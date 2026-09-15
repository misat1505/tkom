use inkwell::values::PointerValue;
use inkwell::{AddressSpace, IntPredicate};

use super::Compiler;
use crate::{
    backend::llvm::llvm_alu::llvm_value::{
        LlvmValue, ENUM_PAYLOAD, ENUM_REFCOUNT, ENUM_TAG, STR_DATA, STR_REFCOUNT, VEC_DATA, VEC_LENGTH, VEC_REFCOUNT,
    },
    common::{
        errors::{CompilerError, ErrorSeverity, IError},
        span::Span,
        types::Type,
    },
    frontend::ast::Expression,
};

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    // ========================================================================
    // Scope tracking
    // ========================================================================

    pub(in crate::backend::llvm::compiler) fn push_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    pub(in crate::backend::llvm::compiler) fn declare_scoped_variable(&mut self, name: String, ptr: PointerValue<'ctx>, ty: Type) {
        self.variables.insert(name.clone(), (ptr, ty.clone()));

        if let Some(scope) = self.scopes.last_mut() {
            scope.push((name, ptr, ty));
        }
    }

    pub(in crate::backend::llvm::compiler) fn pop_scope_and_release(&mut self, span: Span) -> Result<(), Box<dyn IError>> {
        let scope = self.scopes.pop().unwrap_or_default();

        let has_terminator = self.builder.get_insert_block().and_then(|block| block.get_terminator()).is_some();

        for (name, ptr, ty) in scope.iter().rev() {
            self.variables.remove(name);

            if !has_terminator {
                let value = self.load_owned_variable(*ptr, ty, span)?;
                self.release_value(&value, span)?;
            }
        }

        Ok(())
    }

    pub(in crate::backend::llvm::compiler) fn release_scopes_from(&mut self, depth: usize, span: Span) -> Result<(), Box<dyn IError>> {
        for scope_index in (depth..self.scopes.len()).rev() {
            let vars = self.scopes[scope_index].clone();

            for (_, ptr, ty) in vars.iter().rev() {
                let value = self.load_owned_variable(*ptr, ty, span)?;
                self.release_value(&value, span)?;
            }
        }

        Ok(())
    }

    pub(in crate::backend::llvm::compiler) fn release_all_scopes(&mut self, span: Span) -> Result<(), Box<dyn IError>> {
        self.release_scopes_from(0, span)
    }

    fn load_owned_variable(&mut self, ptr: PointerValue<'ctx>, ty: &Type, span: Span) -> Result<LlvmValue<'ctx>, Box<dyn IError>> {
        let err = Self::builder_err(span);

        let llvm_type = LlvmValue::type_to_basic_type_enum(ty, self.context).ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Compiling values of type '{}' is not yet supported.", ty),
                span,
            )) as Box<dyn IError>
        })?;

        let raw = self.builder.build_load(llvm_type, ptr, "scope.drop.load").map_err(&err)?;

        Ok(LlvmValue::from_basic_value_enum(raw, ty))
    }

    pub(in crate::backend) fn expr_needs_retain(expr: &Expression) -> bool {
        matches!(expr, Expression::Variable(_))
    }

    // ========================================================================
    // Retain / release
    // ========================================================================

    pub(in crate::backend) fn expr_needs_release(expr: &Expression) -> bool {
        !matches!(expr, Expression::Variable(_))
    }

    pub(in crate::backend) fn expr_needs_release_in_function_call(expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Literal(_)
                | Expression::FunctionCall { .. }
                | Expression::Index { .. }
                | Expression::FieldAccess { .. }
                | Expression::Casting { .. }
        )
    }

    /// Increments the refcount of a heap-allocated value. No-op for
    /// primitives.
    pub(in crate::backend) fn retain_value(&mut self, value: &LlvmValue<'ctx>, span: Span) -> Result<(), Box<dyn IError>> {
        let err = Self::builder_err(span);

        match value {
            LlvmValue::Str(ptr) => {
                let header_type = LlvmValue::str_header_type(self.context);
                self.bump_refcount(header_type, *ptr, STR_REFCOUNT, 1, &err)?;
            }

            LlvmValue::Vector(ptr, _) => {
                let header_type = LlvmValue::vector_struct_type(self.context);
                self.bump_refcount(header_type, *ptr, VEC_REFCOUNT, 1, &err)?;
            }

            LlvmValue::Struct(ptr, ty) => {
                let Type::Struct { identifier, .. } = ty.as_ref() else {
                    return Ok(());
                };

                let (struct_type, _) = self.struct_llvm_type(identifier, span)?;
                let rc_index = Self::struct_refcount_field_index(struct_type);

                self.bump_refcount(struct_type, *ptr, rc_index, 1, &err)?;
            }

            // BUG FIX: this arm was missing, so enum values were never
            // retained - the second owner's refcount never went up, so the
            // first `release` could free the enum while someone else still
            // held it (use-after-free / segfault).
            LlvmValue::Enum(ptr, ty) => {
                let Type::Enum { identifier, .. } = ty.as_ref() else {
                    return Ok(());
                };

                let (header_type, _variant_indices, _ordered_variants) = self.enum_llvm_type(identifier, span)?;

                self.bump_refcount(header_type, *ptr, ENUM_REFCOUNT, 1, &err)?;
            }

            _ => {}
        }

        Ok(())
    }

    fn bump_refcount(
        &self,
        header_type: inkwell::types::StructType<'ctx>,
        ptr: PointerValue<'ctx>,
        field_index: u32,
        delta: i64,
        err: &impl Fn(inkwell::builder::BuilderError) -> Box<dyn IError>,
    ) -> Result<(), Box<dyn IError>> {
        let i64_type = self.context.i64_type();

        let rc_field = self.builder.build_struct_gep(header_type, ptr, field_index, "rc.field").map_err(err)?;

        let rc = self.builder.build_load(i64_type, rc_field, "rc.val").map_err(err)?.into_int_value();

        let updated = if delta >= 0 {
            self.builder
                .build_int_add(rc, i64_type.const_int(delta as u64, false), "rc.inc")
                .map_err(err)?
        } else {
            self.builder
                .build_int_sub(rc, i64_type.const_int((-delta) as u64, false), "rc.dec")
                .map_err(err)?
        };

        self.builder.build_store(rc_field, updated).map_err(err)?;

        Ok(())
    }

    /// Decrements the refcount of a heap-allocated value, freeing it (and,
    /// recursively, any owned contents) once it reaches zero. No-op for
    /// primitives.
    pub(in crate::backend) fn release_value(&mut self, value: &LlvmValue<'ctx>, span: Span) -> Result<(), Box<dyn IError>> {
        match value {
            LlvmValue::Str(ptr) => self.release_str(*ptr, span),
            LlvmValue::Vector(ptr, inner) => self.release_vector(*ptr, inner, span),
            LlvmValue::Struct(ptr, ty) => self.release_struct(*ptr, ty, span),
            LlvmValue::Enum(ptr, ty) => self.release_enum(*ptr, ty, span),
            _ => Ok(()),
        }
    }

    fn with_last_reference<F>(
        &mut self,
        header_type: inkwell::types::StructType<'ctx>,
        ptr: PointerValue<'ctx>,
        refcount_index: u32,
        span: Span,
        block_prefix: &str,
        body: F,
    ) -> Result<(), Box<dyn IError>>
    where
        F: FnOnce(&mut Self) -> Result<(), Box<dyn IError>>,
    {
        let err = Self::builder_err(span);
        let i64_type = self.context.i64_type();
        let function = self.current_function();

        let rc_field = self
            .builder
            .build_struct_gep(header_type, ptr, refcount_index, "rc.field")
            .map_err(&err)?;

        let rc = self.builder.build_load(i64_type, rc_field, "rc.val").map_err(&err)?.into_int_value();

        let one = i64_type.const_int(1, false);

        let is_last = self.builder.build_int_compare(IntPredicate::EQ, rc, one, "rc.is_last").map_err(&err)?;

        let free_block = self.context.append_basic_block(function, &format!("{}.free", block_prefix));
        let dec_block = self.context.append_basic_block(function, &format!("{}.dec", block_prefix));
        let merge_block = self.context.append_basic_block(function, &format!("{}.merge", block_prefix));

        self.builder.build_conditional_branch(is_last, free_block, dec_block).map_err(&err)?;

        self.builder.position_at_end(free_block);
        body(self)?;
        self.branch_if_no_terminator(merge_block, span)?;

        self.builder.position_at_end(dec_block);
        let decremented = self.builder.build_int_sub(rc, one, "rc.dec").map_err(&err)?;
        self.builder.build_store(rc_field, decremented).map_err(&err)?;
        self.branch_if_no_terminator(merge_block, span)?;

        self.builder.position_at_end(merge_block);

        Ok(())
    }

    fn release_str(&mut self, ptr: PointerValue<'ctx>, span: Span) -> Result<(), Box<dyn IError>> {
        let header_type = LlvmValue::str_header_type(self.context);
        let ptr_type = self.context.ptr_type(AddressSpace::default());

        self.with_last_reference(header_type, ptr, STR_REFCOUNT, span, "str.release", move |compiler| {
            let err = Self::builder_err(span);

            let data_field = compiler
                .builder
                .build_struct_gep(header_type, ptr, STR_DATA, "str.data.field")
                .map_err(&err)?;

            let data = compiler
                .builder
                .build_load(ptr_type, data_field, "str.data.val")
                .map_err(&err)?
                .into_pointer_value();

            compiler
                .builder
                .build_call(compiler.libc.free_fn, &[data.into()], "str.data.free")
                .map_err(&err)?;
            compiler
                .builder
                .build_call(compiler.libc.free_fn, &[ptr.into()], "str.header.free")
                .map_err(&err)?;

            Ok(())
        })
    }

    fn release_vector(&mut self, ptr: PointerValue<'ctx>, inner_type: &Type, span: Span) -> Result<(), Box<dyn IError>> {
        let header_type = LlvmValue::vector_struct_type(self.context);
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();

        let inner_type = inner_type.clone();
        let element_is_owned = matches!(inner_type, Type::Str | Type::Vector(_) | Type::Struct { .. } | Type::Enum { .. });

        self.with_last_reference(header_type, ptr, VEC_REFCOUNT, span, "vec.release", move |compiler| {
            let err = Self::builder_err(span);

            let data_field = compiler
                .builder
                .build_struct_gep(header_type, ptr, VEC_DATA, "vec.data.field")
                .map_err(&err)?;
            let length_field = compiler
                .builder
                .build_struct_gep(header_type, ptr, VEC_LENGTH, "vec.length.field")
                .map_err(&err)?;

            let data = compiler
                .builder
                .build_load(ptr_type, data_field, "vec.data.val")
                .map_err(&err)?
                .into_pointer_value();

            let length = compiler
                .builder
                .build_load(i64_type, length_field, "vec.length.val")
                .map_err(&err)?
                .into_int_value();

            if element_is_owned {
                let element_llvm_type = LlvmValue::type_to_basic_type_enum(&inner_type, compiler.context).ok_or_else(|| {
                    Box::new(CompilerError::at(
                        ErrorSeverity::HIGH,
                        format!("Compiling vectors of type '{}' is not yet supported.", inner_type),
                        span,
                    )) as Box<dyn IError>
                })?;

                let function = compiler.current_function();

                let cond_block = compiler.context.append_basic_block(function, "vec.release.cond");
                let body_block = compiler.context.append_basic_block(function, "vec.release.body");
                let after_block = compiler.context.append_basic_block(function, "vec.release.after");

                let index_alloca = compiler.builder.build_alloca(i64_type, "vec.release.i").map_err(&err)?;
                compiler.builder.build_store(index_alloca, i64_type.const_int(0, false)).map_err(&err)?;

                compiler.builder.build_unconditional_branch(cond_block).map_err(&err)?;

                compiler.builder.position_at_end(cond_block);
                let idx = compiler
                    .builder
                    .build_load(i64_type, index_alloca, "vec.release.i.val")
                    .map_err(&err)?
                    .into_int_value();
                let cmp = compiler
                    .builder
                    .build_int_compare(IntPredicate::SLT, idx, length, "vec.release.cmp")
                    .map_err(&err)?;
                compiler.builder.build_conditional_branch(cmp, body_block, after_block).map_err(&err)?;

                compiler.builder.position_at_end(body_block);
                let elem_ptr = unsafe {
                    compiler
                        .builder
                        .build_gep(element_llvm_type, data, &[idx], "vec.release.elem")
                        .map_err(&err)?
                };
                let elem_raw = compiler
                    .builder
                    .build_load(element_llvm_type, elem_ptr, "vec.release.elem.val")
                    .map_err(&err)?;
                let elem_value = LlvmValue::from_basic_value_enum(elem_raw, &inner_type);
                compiler.release_value(&elem_value, span)?;

                let next = compiler
                    .builder
                    .build_int_add(idx, i64_type.const_int(1, false), "vec.release.i.next")
                    .map_err(&err)?;
                compiler.builder.build_store(index_alloca, next).map_err(&err)?;
                compiler.builder.build_unconditional_branch(cond_block).map_err(&err)?;

                compiler.builder.position_at_end(after_block);
            }

            compiler
                .builder
                .build_call(compiler.libc.free_fn, &[data.into()], "vec.data.free")
                .map_err(&err)?;
            compiler
                .builder
                .build_call(compiler.libc.free_fn, &[ptr.into()], "vec.header.free")
                .map_err(&err)?;

            Ok(())
        })
    }

    fn release_struct(&mut self, ptr: PointerValue<'ctx>, ty: &Type, span: Span) -> Result<(), Box<dyn IError>> {
        let Type::Struct { identifier, fields } = ty else {
            return Ok(());
        };

        let (struct_type, field_indices) = self.struct_llvm_type(identifier, span)?;
        let rc_index = Self::struct_refcount_field_index(struct_type);

        let mut owned_fields: Vec<(u32, Type)> = Vec::new();

        for (field_name, field_index) in field_indices.iter() {
            if let Some(declared_type) = fields.get(field_name) {
                let resolved = self.resolve_type(declared_type);

                if matches!(resolved, Type::Str | Type::Vector(_) | Type::Struct { .. } | Type::Enum { .. }) {
                    owned_fields.push((*field_index, resolved));
                }
            }
        }

        self.with_last_reference(struct_type, ptr, rc_index, span, "struct.release", move |compiler| {
            let err = Self::builder_err(span);

            for (field_index, field_type) in owned_fields.iter() {
                let field_llvm_type = LlvmValue::type_to_basic_type_enum(field_type, compiler.context)
                    .expect("Str, Vector, Struct and Enum always map to a pointer type");

                let field_ptr = compiler
                    .builder
                    .build_struct_gep(struct_type, ptr, *field_index, "struct.release.field")
                    .map_err(&err)?;

                let field_raw = compiler
                    .builder
                    .build_load(field_llvm_type, field_ptr, "struct.release.field.val")
                    .map_err(&err)?;

                let field_value = LlvmValue::from_basic_value_enum(field_raw, field_type);
                compiler.release_value(&field_value, span)?;
            }

            compiler
                .builder
                .build_call(compiler.libc.free_fn, &[ptr.into()], "struct.free")
                .map_err(&err)?;

            Ok(())
        })
    }

    /// Releases an `EnumHeader`: once the refcount hits zero, the active
    /// variant's payload (if it owns a heap value) is released based on the
    /// runtime tag, then the header itself is freed.
    ///
    /// Tag values come from `enum_variants_in_order`'s position (see that
    /// function's doc comment for why `Type::Enum::fields`, a `HashMap`,
    /// can't be used for this).
    fn release_enum(&mut self, ptr: PointerValue<'ctx>, ty: &Type, span: Span) -> Result<(), Box<dyn IError>> {
        let Type::Enum { identifier, .. } = ty else {
            return Ok(());
        };

        let (header_type, _variant_indices, ordered_variants) = self.enum_llvm_type(identifier, span)?;
        let i64_type = self.context.i64_type();

        let mut owned_variants: Vec<(u32, Type)> = Vec::new();

        for (tag, (_, payload_type)) in ordered_variants.iter().enumerate() {
            if let Some(resolved) = payload_type {
                if matches!(resolved, Type::Str | Type::Vector(_) | Type::Struct { .. } | Type::Enum { .. }) {
                    owned_variants.push((tag as u32, resolved.clone()));
                }
            }
        }

        self.with_last_reference(header_type, ptr, ENUM_REFCOUNT, span, "enum.release", move |compiler| {
            let err = Self::builder_err(span);

            if !owned_variants.is_empty() {
                let tag_field = compiler
                    .builder
                    .build_struct_gep(header_type, ptr, ENUM_TAG, "enum.release.tag.field")
                    .map_err(&err)?;
                let tag_val = compiler
                    .builder
                    .build_load(i64_type, tag_field, "enum.release.tag.val")
                    .map_err(&err)?
                    .into_int_value();

                let payload_field = compiler
                    .builder
                    .build_struct_gep(header_type, ptr, ENUM_PAYLOAD, "enum.release.payload.field")
                    .map_err(&err)?;

                let function = compiler.current_function();
                let after_block = compiler.context.append_basic_block(function, "enum.release.after");

                let mut cases = Vec::with_capacity(owned_variants.len());
                let mut variant_blocks = Vec::with_capacity(owned_variants.len());

                for (tag, payload_type) in owned_variants.iter() {
                    let block = compiler.context.append_basic_block(function, &format!("enum.release.variant_{}", tag));
                    cases.push((i64_type.const_int(*tag as u64, false), block));
                    variant_blocks.push((block, payload_type.clone()));
                }

                compiler.builder.build_switch(tag_val, after_block, &cases).map_err(&err)?;

                for (block, payload_type) in variant_blocks {
                    compiler.builder.position_at_end(block);

                    let payload_llvm_type = LlvmValue::type_to_basic_type_enum(&payload_type, compiler.context)
                        .expect("Str, Vector, Struct and Enum payloads always map to a pointer type");

                    let payload_raw = compiler
                        .builder
                        .build_load(payload_llvm_type, payload_field, "enum.release.payload.val")
                        .map_err(&err)?;

                    let payload_value = LlvmValue::from_basic_value_enum(payload_raw, &payload_type);
                    compiler.release_value(&payload_value, span)?;

                    compiler.branch_if_no_terminator(after_block, span)?;
                }

                compiler.builder.position_at_end(after_block);
            }

            compiler
                .builder
                .build_call(compiler.libc.free_fn, &[ptr.into()], "enum.header.free")
                .map_err(&err)?;

            Ok(())
        })
    }
}

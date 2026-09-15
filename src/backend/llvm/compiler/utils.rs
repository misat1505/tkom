use std::collections::HashMap;

use inkwell::{context::Context, types::StructType};

use super::Compiler;
use crate::{
    backend::llvm::llvm_alu::llvm_value::LlvmValue,
    common::{
        errors::{CompilerError, ErrorSeverity, IError},
        span::Span,
        types::Type,
    },
    frontend::ast::{DeclaredType, StructDeclaration},
};

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    pub(in crate::backend) fn resolve_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Unresolved(name) => self.program.types.get(name).cloned().unwrap_or_else(|| ty.clone()),
            Type::Vector(inner) => Type::Vector(Box::new(self.resolve_type(inner))),
            other => other.clone(),
        }
    }

    fn struct_declaration(&self, identifier: &str, span: Span) -> Result<&'a StructDeclaration, Box<dyn IError>> {
        let declared = self.program.declared_types.get(identifier).ok_or_else(|| {
            Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("Unknown struct type '{}'.", identifier),
                span,
            )) as Box<dyn IError>
        })?;

        #[allow(irrefutable_let_patterns)]
        let DeclaredType::Struct(struct_decl) = &declared.value
        else {
            return Err(Box::new(CompilerError::at(
                ErrorSeverity::HIGH,
                format!("'{}' is not a struct type.", identifier),
                span,
            )));
        };

        Ok(struct_decl)
    }

    /// Builds the LLVM layout for a declared struct type.
    ///
    /// The returned `StructType` has one extra trailing `i64` field appended
    /// after all user-declared fields: the refcounting runtime's `refcount`
    /// counter. Use `struct_refcount_field_index` to find it. `field_indices`
    /// only contains the user-declared fields (unaffected by the extra field).
    pub(in crate::backend::llvm::compiler) fn struct_llvm_type(
        &self,
        identifier: &str,
        span: Span,
    ) -> Result<(StructType<'ctx>, HashMap<String, u32>), Box<dyn IError>> {
        let struct_decl = self.struct_declaration(identifier, span)?;

        let mut field_types = Vec::with_capacity(struct_decl.members.len() + 1);
        let mut field_indices = HashMap::new();

        for (idx, member) in struct_decl.members.iter().enumerate() {
            let resolved_type = self.resolve_type(&member.value.member_type.value);

            let llvm_type = LlvmValue::type_to_basic_type_enum(&resolved_type, self.context).ok_or_else(|| {
                Box::new(CompilerError::at(
                    ErrorSeverity::HIGH,
                    format!("Compiling struct fields of type '{}' is not yet supported.", resolved_type),
                    member.span,
                )) as Box<dyn IError>
            })?;

            field_types.push(llvm_type);
            field_indices.insert(member.value.identifier.value.clone(), idx as u32);
        }

        // Trailing refcount field, not exposed in `field_indices`.
        field_types.push(self.context.i64_type().into());

        Ok((self.context.struct_type(&field_types, false), field_indices))
    }

    /// `EnumHeader { refcount: i64, tag: i64, payload: [word_count x i64] }`.
    ///
    /// Field order matches the `ENUM_*` index constants above.
    ///
    /// `variants` is `(variant_name, payload_type)` - one optional payload
    /// type per variant, matching `Type::Enum`'s single-payload-per-variant
    /// shape (`InProgress(Deadline)`, `Aborted` with no value, etc). This is
    /// NOT the declaration order from the AST/`Type::Enum` HashMap (which has
    /// no stable order) - callers must pass variants pre-sorted into the
    /// same canonical order everywhere (see `Compiler::enum_llvm_type`),
    /// since the position in this slice becomes the runtime tag value that
    /// both enum-literal and match codegen must agree on.
    ///
    /// `payload_size` is the byte size of the largest variant's payload type;
    /// a variant with no payload (e.g. a bare `Aborted`) contributes 0 and
    /// never grows it. It's rounded up to whole 8-byte words so the payload
    /// field is naturally 8-byte aligned - required since every supported
    /// field type (ints up to i64, f64, and heap pointers) needs up to 8-byte
    /// alignment, which a `[N x i8]` field would not guarantee.
    pub(in crate::backend::llvm::compiler) fn enum_struct_type(
        &self,
        variants: &[(String, Option<Type>)],
        context: &'ctx Context,
        span: Span,
    ) -> Result<StructType<'ctx>, Box<dyn IError>> {
        let i64_type = context.i64_type();

        let mut payload_size: u64 = 0;
        for (_, payload_ty) in variants {
            let variant_size = match payload_ty {
                Some(ty) => LlvmValue::element_byte_size(&self.resolve_type(ty), i64_type, span)?
                    .get_zero_extended_constant()
                    .expect("element_byte_size always returns a constant int"),
                None => 0,
            };
            payload_size = payload_size.max(variant_size);
        }

        let word_count = (payload_size + 7) / 8;
        let payload_type = i64_type.array_type(word_count as u32);

        Ok(context.struct_type(
            &[
                i64_type.into(),     // refcount
                i64_type.into(),     // tag
                payload_type.into(), // payload (union storage, see ENUM_PAYLOAD)
            ],
            false,
        ))
    }

    /// Index of the trailing `refcount: i64` field appended by `struct_llvm_type`.
    pub(in crate::backend::llvm::compiler) fn struct_refcount_field_index(struct_type: StructType<'ctx>) -> u32 {
        struct_type.count_fields() - 1
    }
}

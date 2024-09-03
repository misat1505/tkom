# Interpreter for custom programming language

This documentation covers the interpreter for a custom programming language with a focus on strong static typing and clear variable scoping. The interpreter handles various data types such as integers (i64), floating-point numbers (f64), strings (str), and booleans (bool). It supports key features like mutable variables, type conversions, and control flow constructs including if, for, and switch statements.

## Functionality

The language is strongly and statically typed.

1. Supported Data Types:

   - i64 (integers)
   - f64 (floating-point numbers)
   - str (strings)
   - bool (true / false)
   - void (no return value from a function)

2. Variables:

   - Can hold one of the aforementioned types except void.
   - All variables are mutable.
   - Variables are visible only within the block where they are declared.
   - Variables of a specific type can be declared without initializing them. In such cases, the default value for that type will be assigned.

3. Variable Operations:

   - Assignment (=)
   - Arithmetic (+, -, \*, /)
   - Comparisons (==, <, <=, >, >=, !=)
   - Logical operators (||, &&)

4. Type Conversion:

   - i64 and f64 can be cast to each other, to strings, and to boolean (if <= 0, it will be false, otherwise true).
   - Strings can be cast to i64 and f64 with error reporting, and to boolean (an empty string means false, otherwise true).

5. Functions:

   - Can accept parameters by value and by reference.
   - Can return a value of a specified type (functions may also return nothing).
   - Functions can be called recursively.

6. If Statement:

   - Optional else.

7. For Loop:

   - The iterator does not need to be declared or updated.
   - The declared iterator is not visible outside the for loop.

8. Switch Statement (pattern matching):

   - Allows declaring a variable visible only within the switch.
   - Each block where the condition is met is executed.
   - Premature exit is possible using break.

9. Built-in Functions:
   - `print(text)`: prints a string to standard output with a newline character.
   - `input(text)`: prints a string to standard output and waits for user input, returning a string.
   - `mod(a, b)`: takes two numbers and returns the value of `a % b`.

## Language Examples

1. Function that checks if a given number is a prime number. It also receives an iteration count by reference.

```
fn is_prime(i64 x, &i64 total_iters): bool {
  if (x < 2) {
    return false;
  }

  for (i64 i = 2; i < x; i = i + 1) {
    total_iters = total_iters + 1;
    if (mod(x, i) == 0) {
      return false;
    }
  }

  return true;
}
```

2. Function that calculates the nth term of the Fibonacci sequence using an iterative method.

```
fn fib_iter(i64 x): i64 {
  i64 prev2 = 1;
  i64 prev1 = 1;
  i64 total;

  if (x == 1 || x == 2) {
    return 1;
  }

  for (i64 i = 2; i < x; i = i + 1) {
    total = prev1 + prev2;
    prev2 = prev1;
    prev1 = total;
  }

  return total;
}
```

3. Function that calculates the nth term of the Fibonacci sequence using a recursive method.

```
fn fib_rec(i64 x): i64 {
  if (x == 1 || x == 2) {
    return 1;
  }

  return fib_rec(x - 1) + fib_rec(x - 2);
}
```

4. Program that asks the user for a number and displays information about its sign.

```
switch (input("Pick a number: ") as i64: x) {
  (x < 0) -> {
    print("Negative");
  }
  (x == 0) -> {
    print("Zero");
  }
  (x > 0) -> {
    print("Positive");
  }
}
```

## Grammar

### Syntax Part

**program** = { function_declaration | assign_or_call | if_statement | for_statement | switch_statement | declaration, ";" };

**comment** = "#" , {unicode_character - "\n"}, "\n";

**function_declaration** = “fn”, identifier, "(", parameters, ")", “:”, type | “void”, statement_block;

```
fn is_prime(i64 x, &i64 total_iters): bool {
    return true;
}
```

**parameters** = [ parameter, { ",", parameter } ];

**parameter** = [“&”], type, identifier;

**statement_block** = "{", {statement}, "}";

**statement** = assign_or_call | if_statement | for_statement | switch_statement | declaration, ";" | return_statement | break_statement;

**assign_or_call** = identifier, ("=", expression | "(", arguments, ")"), ";";

```
x = 5;
my_fun(5, 2);
```

**declaration** = type, identifier, [ "=", expression ];

```
bool is_valid = true;
```

**if_statement** = "if", "(", expression, ")", statement_block, [ "else", statement_block ];

```
if (x == 5) {} else {}
```

**for_statement** = "for", "(", [ declaration ], “;”, expression, “;”, [ identifier, "=", expression ], ")", statement_block;

```
for (i64 i = 0; i < 10; i = i + 1) {}
```

```
i64 i = 0
for (; i < 10 ;) {
    i = i + 1;
}
```

**break_statement** = "break", ";";

```
break;
```

**return_statement** = "return", [ expression ], ";";

```
return a + 2 * b;
```

**argument** = [“&”], expression;

**arguments** = [ argument, {",", argument} ];

```
a + 2, &b, c
```

**expression** = concatenation_term { “||”, concatenation_term };

```
a == b && b || c
```

**concatenation_term** = relation_term, { “&&”, relation_term };

```
a == b && b
```

**relation_term** = additive_term, [ relation_operands, additive_term ];

```
x == y
```

**additive_term** = multiplicative_term , { ("+" | "-"), multiplicative_term };

```
1 + (1 + 2) / (2 + 3)
```

**multiplicative_term** = casted_term, { ("\*" | "/"), casted_term };

```
(1 + 2) / (2 + 3)
```

**casted_term** = unary_term, [ “as”, type ];

```
(x + add(2, 2)) as f64
2 as i64                # 2
2 as f64                # 2.0
2 as str                # “2”
-2 as str               # "-2"
2 as bool               # true
0 as bool               # false
“123” as i64            # 123
“fdsfs” as i64          # error
“” as bool              # false
“a” as bool             # true
```

**unary_term** = [ ("-", "!") ], factor;

```
-2
-(x + 5)
!true
```

**factor** = literal | ( "(", expression, ")" ) | identifier_or_call;

```
5
(2.2 + 3 as f64)
x
fun(5)
```

**identifier_or_call** = identifier, [ "(", arguments, ")" ];

```
x
fun(5)
```

**literal** = integer_literal | float_literal | boolean_literal | string_literal;

**identifier** = letter, {character};

```
super_variable_123
```

**switch_statement** = "switch", "(", switch_expressions, ")", "{", {switch_case}, "}";

**switch_expression** = expression, [ ":", identifier ];

**switch_expressions** = switch_expression, { “,”, switch_expression };

**switch_case** = "(", expression, ")", "->", statement_block;

```
switch (x: temp1, y: temp2) {
    (x < 5 && temp2 < 5) -> {
      print("Less than 5.");
    }
    (temp1 < 10 && y < 10) -> {
      print("Less than 10.");
      break;
    }
}
```

### Lexical Part

**letter** = "a" - "z" | "A" - "Z";

**type** = “i64“| “f64” | “bool” | “str”;

**relation_operands** = "==" | "<" | "<=" | ">" | ">=" | "!=";

**digit** = "0" - “9”;

**non_zero_digit** = "1" - "9";

**integer_literal** = ( non_zero_digit, {digit} ) | “0”;

```
1, 12, 10, 0
```

**float_literal** = integer_literal, ".", {digit}

```
1.0, 1.2, 10.0, 0.0, 0.00001;
```

**string_literal** = “\””, {unicode_character - “\””}, “\””;

**boolean_literal** = “true” | “false”;

**character** = "a" - "z" | "A" - "Z" | "0" - "9" | "\_";

**unicode_character** = (all unicode characters)

## Operator priority

<table>
  <tr>
   <td>operator
   </td>
   <td>priority
   </td>
  </tr>
  <tr>
   <td>- (number negetion)
   </td>
   <td>7
   </td>
  </tr>
  <tr>
   <td>!
   </td>
   <td>7
   </td>
  </tr>
  <tr>
   <td>as
   </td>
   <td>6
   </td>
  </tr>
  <tr>
   <td>*
   </td>
   <td>5
   </td>
  </tr>
  <tr>
   <td>/
   </td>
   <td>5
   </td>
  </tr>
  <tr>
   <td>+
   </td>
   <td>4
   </td>
  </tr>
  <tr>
   <td>- (subtraction)
   </td>
   <td>4
   </td>
  </tr>
  <tr>
   <td>>
   </td>
   <td>3
   </td>
  </tr>
  <tr>
   <td>>=
   </td>
   <td>3
   </td>
  </tr>
  <tr>
   <td><
   </td>
   <td>3
   </td>
  </tr>
  <tr>
   <td><=
   </td>
   <td>3
   </td>
  </tr>
  <tr>
   <td>==
   </td>
   <td>3
   </td>
  </tr>
  <tr>
   <td>!=
   </td>
   <td>3
   </td>
  </tr>
  <tr>
   <td>&&
   </td>
   <td>2
   </td>
  </tr>
  <tr>
   <td>||
   </td>
   <td>1
   </td>
  </tr>
</table>

## Error Handling

Errors and Warnings Division:

1. **Errors:** Reported to the higher-level component, indicating that execution cannot continue.
2. **Warnings:** The higher-level component creates a function to be executed in case of a warning; execution is not halted.

### Lexer Errors

The lexer reports an error when it cannot map a given string to any token. Additionally, it catches overflows for numbers entered by users and excessively long comments and identifiers.

```
Overflow occurred while parsing integer
At line: 13, column: 28

At line:
i64 a = 7647326473264873264873264;
                           ^
```

The lexer reports warnings when it can infer what the user likely intended but the input was not correctly written.

```
Expected '|'
At line: 18, column: 15
```

```
String not closed
At line: 21, column: 37
```

### Parser Errors

The parser reports an error when it encounters a token that does not match the language grammar specification or when it detects function redeclaration.

```
Couldn't create statement block while parsing if statement.
At line: 13, column: 12.
```

```
Redeclaration of function 'print'.
At: line: 14, column: 1.
```

### Semantic Analyzer Errors

The semantic analyzer reports an error when it finds a function call in the parse tree for a non-existent function, with the wrong number of arguments, or with arguments passed incorrectly.

```
Invalid number of arguments for function 'foo'. Expected 1, given 0.
At line: 18, column: 1.
```

```
Parameter 'x' in function 'foo' passed by Reference - should be passed by Value.
At line: 19, column: 6.
```

### Interpreter Errors

The interpreter reports an error when it encounters an illegal operation, such as:

- Adding values of different types,
- Assigning a value of a different type to a variable,
- Passing an incorrect type as a function argument,
- Returning an incorrect type from a function,
- Redeclaring a variable,
- Conditions in if, switch, for blocks not being of type bool,
- Using break outside of a for or switch,
- Using return outside of a function,
- Stack overflow due to function calls,
- Arithmetic overflow,
- Type conversion errors.

```
Cannot perform addition between values of type 'i64' and 'f64'.
At line: 13, column: 13.
```

```
Cannot assign value of type 'str' to variable 's' of type 'i64'.
At line: 18, column: 9.
```

```
Cannot cast String 'abc' to i64.
At: line: 18, column: 9
```

## Execution Instructions

1. **Running in Debug Mode:**

```
cargo run path_to_file
```

2. **Building the Project:**

```
cargo build --release
.\target\release\tkom.exe path_to_file
```

**Analysis of Functional and Non-Functional Requirements**

## Implementation Method

The main components of the program are the lexical analyzer, parser, semantic analyzer, and interpreter.

### Lexical Analyzer

The lexer operates with the `LazyStreamReader` class. `LazyStreamReader` takes an input source and provides 3 methods:

- `current()` - returns the current character
- `next()` - consumes 1 character and returns it
- `position()` - returns the current position

The lexer works lazily - it provides the `generate_token` method. It queries `LazyStreamReader` for subsequent characters and tries to create a token based on them. If it fails to create a token of any category, it reports an error.

### Parser

The primary task of the parser is to create a syntax tree that adheres to the accepted grammar. It queries the lexer for tokens through `generate_token`. The result is a program tree divided into main statements, user-defined function definitions, and built-in functions.

### Semantic Analyzer

Implements the visitor trait, traversing the tree to find function calls and checking their correctness. It does not terminate with an error but stores any errors it finds internally.

### Interpreter

The interpreter implements the visitor trait and executes the program. To facilitate communication between visits, the interpreter includes the following fields:

- `last_result` - stores intermediate computation results,
- `last_arguments` - holds function call arguments (pointers to values),
- Flags `is_breaking` and `is_returning`, which are set during visits to break and return, and are cleared upon encountering structures that allow this.

The interpreter interacts with the `Stack` class, which stores the function call stack. A single `StackFrame` holds an instance of the `ScopeManager` class, which is also a stack but is used for managing variable scopes. Each field in the `ScopeManager` stack (Scope) stores a `HashMap` of variable name -> value pointer. Values are represented by an enumeration `Value`, and operations on them are performed by the `ALU` class.

## Testing Approach

1. **Lexer Tests:**

   - Check if tokens are created correctly.
   - Check if the lexer responds appropriately to incorrect input.

2. **Parser Tests:**

   - Verify the correctness of the parse tree.
   - Check if the parser handles syntax errors correctly.

3. **Interpreter Tests:**

   - Check if the interpreter reacts correctly to given AST trees.

4. **Unit Tests for:**

   - `LazyStreamReader`,
   - `ALU`,
   - `Value`,
   - `Scope` and `ScopeManager`,
   - `Stack`

5. **Integration Tests for the entire project**

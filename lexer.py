import re

# Define token types using regular expressions
TOKEN_TYPES = [
    ('NUMBER', r'\d+'),
    ('IDENTIFIER', r'[a-zA-Z_]\w*'),
    ('PLUS', r'\+'),
    ('MINUS', r'\-'),
    ('TIMES', r'\*'),
    ('DIVIDE', r'/'),
    ('LPAREN', r'\('),
    ('RPAREN', r'\)'),
    ('WHITESPACE', r'\s+'),
    ('EQUALS', r'\='),
]


# Lexer function
def lexer(input_code):
    tokens = []
    i = 0

    while i < len(input_code):
        match = None

        for token_type, pattern in TOKEN_TYPES:
            regex = re.compile(pattern)
            match = regex.match(input_code, i)

            if match:
                value = match.group(0)
                tokens.append((token_type, value))
                i = match.end()
                break

        if not match:
            raise Exception(f"Invalid character at position {i}: {input_code[i]}")

    return tokens


# Example usage
code = "my_variable = 3 + 4 * (5 - 2)"
result = lexer(code)

for token in result:
    print(token)

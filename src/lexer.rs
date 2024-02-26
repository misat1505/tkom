use regex::Regex;

const TOKEN_TYPES: &[(&str, &str)] = &[
    ("NUMBER", r"^\d+"),
    ("IDENTIFIER", r"^[a-zA-Z_]\w*"),
    ("PLUS", r"^\+"),
    ("MINUS", r"^\-"),
    ("TIMES", r"^\*"),
    ("DIVIDE", r"^/"),
    ("LPAREN", r"^\("),
    ("RPAREN", r"^\)"),
    ("WHITESPACE", r"^\s+"),
    ("EQUALS", r"^\="),
    ("STRING", r#""(?:[^"\\]|\\.)*""#),
    ("NEWLINE", r#"^(\n)"#),
];

pub fn lexer(input_code: &str) -> Vec<(String, String)> {
    let mut tokens: Vec<(String, String)> = Vec::new();
    let mut iterator = 0;

    while iterator < input_code.len() {
        let mut match_found = false;

        for (token_type, pattern) in TOKEN_TYPES {
            let regex = Regex::new(&pattern).unwrap();
            if let Some(mat) = regex.find(&input_code[iterator..]) {
                let value = mat.as_str().to_string();
                tokens.push((token_type.to_string(), value));
                iterator += mat.end();
                match_found = true;
                break;
            }
        }

        if !match_found {
            panic!(
                "Invalid character at position {}: {}",
                iterator,
                &input_code[iterator..=iterator]
            );
        }
    }

    tokens
}

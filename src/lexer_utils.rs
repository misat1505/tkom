#[derive(Debug, Clone)]
pub enum LexerIssueKind {
    WARNING,
    ERROR,
}

#[derive(Debug, Clone)]
pub struct LexerIssue {
    pub kind: LexerIssueKind,
    pub message: String,
}

impl LexerIssue {
    pub fn new(kind: LexerIssueKind, message: String) -> Self {
        LexerIssue { kind, message }
    }
}

pub struct LexerOptions {
    pub max_comment_length: u32,
    pub max_identifier_length: u32,
}

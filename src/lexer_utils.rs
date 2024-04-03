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

pub struct LexerWarningManager {
    issues: Vec<LexerIssue>,
}

impl LexerWarningManager {
    pub fn new() -> Self {
        LexerWarningManager { issues: vec![] }
    }

    pub fn add(&mut self, message: String) {
        self.issues
            .push(LexerIssue::new(LexerIssueKind::WARNING, message));
    }

    pub fn get_warnings(&self) -> Vec<LexerIssue> {
        self.issues.clone()
    }
}

pub struct LexerOptions {
    pub max_comment_length: u32,
    pub max_identifier_length: u32,
}

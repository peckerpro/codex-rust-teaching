//! AST data structures shared by parser and later stages.

use rt_common::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AstNode {
    pub id: usize,
    pub kind: String,
    pub label: String,
    pub span: Span,
    pub children: Vec<AstNode>,
}

impl AstNode {
    pub fn new(id: usize, kind: impl Into<String>, label: impl Into<String>, span: Span) -> Self {
        Self {
            id,
            kind: kind.into(),
            label: label.into(),
            span,
            children: Vec::new(),
        }
    }

    pub fn with_child(mut self, child: AstNode) -> Self {
        self.children.push(child);
        self
    }
}

//! Semantic analysis scaffolding.

use rt_ast::AstNode;
use rt_common::Diagnostic;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SemanticTrace {
    pub symbols: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn analyze(_root: &AstNode) -> SemanticTrace {
    SemanticTrace::default()
}

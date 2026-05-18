//! LLVM IR text generation scaffolding.

use rt_ast::AstNode;

pub fn generate_llvm_ir(_root: &AstNode) -> String {
    "define i32 @main() {\nentry:\n  ret i32 0\n}\n".to_string()
}

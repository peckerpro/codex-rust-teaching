//! Teaching IR projection scaffolding.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TeachingIr {
    pub text: String,
}

pub fn project_from_llvm(llvm_ir: &str) -> TeachingIr {
    TeachingIr {
        text: llvm_ir
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

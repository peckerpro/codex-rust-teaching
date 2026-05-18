# Rust Teaching Compiler

Course-oriented Rust teaching compiler for compiler-principles education.

The project goal is to implement a small, practical Rust subset compiler in
Rust, with visible compiler stages:

- lexical analysis
- syntax analysis
- semantic analysis
- LLVM IR generation
- simplified teaching IR and CFG visualization

The compiler core should keep dependencies minimal and modules loosely coupled.
Testing follows a gray-first strategy: validate the smallest working compiler
path before running full regression tests.

See `docs/` for the current requirements, technology choices, environment notes,
roadmap, architecture decisions, and testing strategy.


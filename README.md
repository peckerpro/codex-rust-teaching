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

## Current Status

The initial Cargo workspace is in place with the teaching compiler crates
described in the roadmap:

- `rt-common`
- `rt-lexer`
- `rt-ast`
- `rt-parser`
- `rt-semantic`
- `rt-codegen`
- `rt-ir-view`
- `rt-driver`
- `rt-cli`

Phase 0 validation:

```bash
cargo build
cargo test
cargo run -p rt-cli -- --help
```

Phase 1 lexer smoke tests:

```bash
cargo run -p rt-cli -- --emit tokens --format json examples/basic.rs
cargo run -p rt-cli -- --emit tokens examples/lexer_showcase.rs
cargo test -p rt-lexer
```

Phase 2 parser smoke tests:

```bash
cargo run -p rt-cli -- --emit ast examples/basic.rs
cargo run -p rt-cli -- --emit ast --format json examples/basic.rs
cargo test -p rt-parser
```

Phase 3 semantic smoke tests:

```bash
cargo run -p rt-cli -- --check examples/basic.rs
cargo run -p rt-cli -- --emit semantic --format json examples/basic.rs
cargo run -p rt-cli -- --emit semantic examples/semantic_errors.rs
cargo test -p rt-semantic
```

Phase 4 LLVM IR smoke tests:

```bash
cargo run -p rt-cli -- -S examples/basic.rs -o examples/basic.ll
opt-18 -passes=verify examples/basic.ll -disable-output
lli-18 examples/basic.ll
cargo run -p rt-cli -- --emit llvm-ir --format json examples/basic.rs
cargo test -p rt-codegen
```

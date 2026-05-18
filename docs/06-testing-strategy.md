# 测试策略：灰度优先，再全量回归

## 1. 总体原则

本项目测试采用“灰度优先”的策略。

灰度测试指：每次修改后，先用最小输入、最短命令验证核心通路没有断，再进入更耗时、更全面的测试。

这样做的原因：

- 编译器项目链路长，直接全量测试失败时定位成本高。
- 教学项目需要快速确认某个阶段是否仍然可解释、可展示。
- LLVM 后端错误可能来自词法、语法、语义或 codegen，灰度测试有助于逐层定位。

## 2. 测试分层

### 2.1 灰度测试

目标：验证最小通路。

示例代码：

```rust
fn main() -> i32 {
    return 42;
}
```

建议命令：

```bash
rtc --emit tokens examples/basic.rs
rtc --emit ast examples/basic.rs
rtc --check examples/basic.rs
rtc -S examples/basic.rs -o target/basic.ll
opt-18 -verify target/basic.ll -disable-output
lli-18 target/basic.ll
```

灰度测试只关注：

- 命令能否运行。
- 是否能输出目标阶段结果。
- 是否能生成合法 LLVM IR。
- 是否能得到预期运行结果。

### 2.2 定向测试

目标：验证某个 crate 或某个阶段。

```bash
cargo test -p rt-lexer
cargo test -p rt-parser
cargo test -p rt-semantic
cargo test -p rt-codegen
```

每个阶段都应包含：

- 正常输入。
- 错误输入。
- 边界输入。

### 2.3 集成测试

目标：验证完整编译链路。

```bash
rtc --emit all --format json examples/basic.rs
rtc -S examples/basic.rs -o target/basic.ll
opt-18 -verify target/basic.ll -disable-output
lli-18 target/basic.ll
```

集成测试需要覆盖：

- 成功编译。
- 语法错误。
- 类型错误。
- 未定义变量。
- 控制流错误。

### 2.4 全量测试

目标：提交前或阶段验收时运行。

```bash
cargo fmt --check
cargo test --workspace
```

后续加入：

```bash
cargo clippy --workspace --all-targets
npm test
npm run build
```

## 3. 推荐开发循环

每次实现一个小功能时：

```text
1. 写或更新一个最小示例。
2. 跑该阶段灰度测试。
3. 跑对应 crate 测试。
4. 跑完整通路测试。
5. 修改文档或示例。
6. 阶段结束时跑全量测试。
```

## 4. LLVM 后端测试顺序

LLVM 后端不要一开始就测试可执行文件，应按以下顺序：

```text
1. 生成 .ll 文本。
2. 人眼检查 .ll 是否简洁可读。
3. 使用 opt-18 -verify 验证 IR 结构。
4. 使用 lli-18 解释执行。
5. 使用 clang-18 链接生成可执行文件。
6. 运行可执行文件并检查退出码或输出。
```

## 5. 可视化测试

可视化 JSON 测试重点不是 UI 漂亮，而是协议稳定。

需要检查：

- Token span 是否能映射回源码。
- AST node id 是否稳定。
- semantic symbol 是否包含作用域信息。
- LLVM IR basic block 是否能生成 CFG。
- diagnostics 是否包含 level、message、span。

第一版可以用快照文件保存 JSON 输出：

```text
tests/snapshots/basic.tokens.json
tests/snapshots/basic.ast.json
tests/snapshots/basic.semantic.json
tests/snapshots/basic.all.json
```


## 6. Rustc Cross-Validation

For supported executable subset examples, compare this compiler against the official Rust compiler by process exit code:

```bash
bash scripts/compare-with-rustc.sh
```

The teaching subset currently accepts `fn main() -> i32`, while official `rustc` does not accept `i32` as the direct Rust `main` return type. The cross-validation script therefore generates a temporary Rust wrapper:

```rust
fn main() {
    std::process::exit(rtc_main());
}
```

and rewrites the example's `fn main` to `fn rtc_main` before compiling with `rustc`. This keeps the student's core program unchanged while making exit-code comparison possible.

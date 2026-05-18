# Rust 教学编译器技术栈

## 1. 总体架构

项目建议采用“编译器核心 + CLI + 可视化 Web 前端 + 可选服务端”的架构。

```text
source.rs
  |
  v
rtc compiler core
  |
  +-- lexer: source -> tokens
  +-- parser: tokens -> AST
  +-- semantic: AST -> symbols/types/diagnostics
  +-- codegen: AST + semantic info -> LLVM IR
  +-- ir-view: LLVM IR/CFG -> teaching IR
  |
  v
JSON trace / text output
  |
  v
HTML visualizer
```

## 2. Rust 编译器核心

| 模块 | 技术 | 说明 |
|---|---|---|
| 语言 | Rust stable | 编译器主体必须使用 Rust 实现 |
| Workspace | Cargo workspace | 拆分多个 crate，便于教学 |
| Lexer | 手写状态机 | 适合讲授词法分析 |
| Parser | 手写递归下降 + Pratt parser | 适合表达式优先级讲解 |
| AST | Rust enum/struct | 每个节点带 `Span` |
| Semantic | 手写符号表和类型检查 | 对应编译原理课程重点 |
| LLVM 后端 | 第一版手写 LLVM IR 文本；后续可选 `inkwell` | 先减少依赖，同时保证可被 LLVM 工具链验证 |
| 教学 IR | 从 LLVM IR/CFG 简化抽取 | 用于课堂可视化，不作为主编译目标 |
| JSON | 第一版手写 JSON 输出；后续可选 `serde` | 减少核心依赖，保持输出协议稳定 |
| CLI | 手写参数解析 | 第一版参数少，避免引入 `clap` |
| Diagnostic | 自定义诊断类型 | 文本诊断和 JSON 诊断分离 |
| 测试 | `cargo test` | 单元测试和集成测试 |

依赖原则：

- 编译器核心第一版只使用 Rust 标准库。
- 前端可以使用 npm 生态，但必须与编译器核心通过 JSON/HTTP 解耦。
- 如果后续引入 `serde`、`clap`、`inkwell` 等依赖，需要在架构决策记录中说明理由。
- LLVM 18 作为系统工具链依赖存在，用于验证和运行生成的 LLVM IR。

## 3. 推荐 crate 划分

```text
crates/
  rt-common/
    Span, SourceFile, Diagnostic, Symbol
  rt-lexer/
    Lexer, Token
  rt-ast/
    AST nodes, visitor
  rt-parser/
    Parser
  rt-semantic/
    NameResolver, TypeChecker, Scope, SymbolTable
  rt-codegen/
    LLVM IR text generation
  rt-ir-view/
    LLVM IR parser/projection, Teaching IR, CFG builder
  rt-driver/
    Compile pipeline, JSON trace
  rt-cli/
    Command line interface
```

早期也可以先用一个 crate 快速启动，但建议在第 2 个里程碑前拆成 workspace。

## 4. HTML 可视化前端

| 层次 | 技术 | 说明 |
|---|---|---|
| 构建工具 | Vite | 简单、快、适合教学平台 |
| 框架 | React + TypeScript | 生态成熟、组件丰富、npm 兼容性好 |
| 编辑器 | Monaco Editor | VS Code 同款编辑器 |
| AST 树 | 自写 Tree View + React | 先保证稳定和可控，后续再引入图组件 |
| 表格 | TanStack Table 或普通 HTML table | Token 和符号表展示 |
| 图可视化 | Mermaid 优先，React Flow 可选 | Mermaid 轻量兼容，适合 CFG 首版 |
| 通信 | HTTP JSON | 本地或云端统一 API |

兼容性策略：

- 前端首选 React + TypeScript + Vite。
- UI 尽量使用标准 HTML/CSS 和少量稳定组件。
- 复杂可视化先用 Mermaid / SVG / Canvas，不绑定重型平台。
- 与编译器通信只依赖稳定 JSON schema，避免前端耦合 Rust 内部结构。

## 5. 本地可视化服务

开发阶段建议增加一个本地 server：

```text
rtc-web-server
```

职责：

- 接收前端提交的源码。
- 调用编译器核心库，而不是 shell 调用 CLI。
- 返回统一 JSON trace。

可选实现：

| 方案 | 优点 | 缺点 |
|---|---|---|
| Rust Axum server | 与编译器共享类型，部署一致 | 需要写 HTTP 服务 |
| Node server 调用 rtc binary | npm 化更自然 | 需要处理进程和临时文件 |
| 前端直接 wasm 调用 | 部署简单 | 后端/LLVM 不适合 wasm |

推荐路线：

1. 第一版先让 CLI 输出 JSON，前端或脚本读取 JSON。
2. 第二版用 Rust 标准库实现一个极简 HTTP server，或用 Node wrapper 调用 `rtc`。
3. 服务端框架如 Axum 暂不作为第一优先级，避免早期依赖过多。
3. 前端阶段可考虑 wasm，但不作为第一优先级。

## 6. IR 技术选择

本项目以 LLVM IR 作为主 IR 和正确性锚点。原因：

- 编译器是否“真的工作”，需要通过真实后端验证。
- LLVM IR 可用 `llvm-as`、`opt`、`llc`、`lli`、`clang` 等工具检查。
- 学生可以看到高级语言逐步下降到真实 IR 的过程。
- 教学 IR 由 LLVM IR/CFG 简化而来，可以避免“只做展示不做编译”的偏差。

LLVM IR 示例：

```llvm
define i32 @main() {
entry:
  ret i32 42
}
```

教学 IR 示例：

```text
fn main() -> i32:
bb0:
  ret 42
```

IR 生成顺序：

```text
AST + Semantic Info
  -> LLVM IR text
  -> LLVM verifier / optional execution
  -> LLVM basic blocks and instructions
  -> simplified Teaching IR
  -> JSON visualization trace
```

命令行为：

```bash
rtc -S main.rs -o main.ll
rtc -c main.rs -o main.o
rtc main.rs -o main
rtc --emit teaching-ir main.rs
rtc --emit all --format json main.rs
```

## 7. 参考项目

可参考但不要直接照搬：

- `rust-lang/rust`：宏观架构、诊断、HIR/MIR 概念。
- `rustc-dev-guide`：理解 rustc 编译阶段。
- Crafting Interpreters：手写 lexer/parser 的教学表达。
- LLVM Kaleidoscope 教程：IR 和 codegen 教学。

本项目要保持“课程可理解”，优先实现小而完整的链路。

# Rust 教学编译器分阶段规划

## 总体原则

本项目无法在一个 session 中完成，因此采用分阶段交付。每个阶段都要形成可运行、可演示、可测试的小闭环。

优先级原则：

1. 先让完整编译链路跑通：源码到 LLVM IR，再到可执行/可运行结果。
2. 每次开发先灰度测试最小通路，再全量测试。
3. 再让每个阶段可视化。
4. 再扩展语言特性。
5. 最后考虑跨平台、npm 分发、云端部署。

工程原则：

- 编译器主体使用 Rust 编写。
- 核心编译器第一版只使用 Rust 标准库。
- 模块之间通过明确数据结构通信，避免跨阶段互相依赖内部实现。
- 前端、服务端、npm 包装层不得污染编译器核心。

## Phase 0: 项目初始化

目标：建立可持续开发的骨架。

任务：

- 创建 Cargo workspace。
- 创建基础 crate：
  - `rt-common`
  - `rt-lexer`
  - `rt-ast`
  - `rt-parser`
  - `rt-semantic`
  - `rt-codegen`
  - `rt-ir-view`
  - `rt-driver`
  - `rt-cli`
- 添加 `examples/basic.rs`。
- 添加 README。
- 添加 `cargo fmt` 和基础测试。
- 不引入第三方 Rust crate。

验收：

```bash
cargo build
cargo test
cargo run -p rt-cli -- --help
```

灰度测试：

```bash
cargo run -p rt-cli -- --help
```

## Phase 1: 词法分析

目标：源码到 Token 流。

支持：

- 关键字：`fn let mut if else while loop break continue return true false`
- 标识符。
- 整数字面量。
- 字符和字符串字面量。
- 运算符和分隔符。
- 注释。
- Span 位置信息。

任务：

- 实现 `Lexer`。
- 实现 `TokenKind`。
- 实现错误 token 和诊断。
- 实现 `rtc --emit tokens`。
- 实现 `rtc --emit tokens --format json`。

验收：

```bash
cargo test -p rt-lexer
rtc --emit tokens examples/basic.rs
rtc --emit tokens --format json examples/basic.rs
```

灰度测试：

```bash
rtc --emit tokens examples/basic.rs
```

## Phase 2: 语法分析

目标：Token 流到 AST。

支持：

- 函数定义。
- 参数列表。
- 块。
- `let` 语句。
- `return`。
- `if / else`。
- `while`。
- 表达式优先级。
- 函数调用。

任务：

- 实现 AST 数据结构。
- 实现递归下降 parser。
- 实现 Pratt 表达式 parser。
- 每个 AST 节点携带 `Span`。
- 实现 `rtc --emit ast`。
- 实现 `rtc --emit ast --format json`。

验收：

```bash
cargo test -p rt-parser
rtc --emit ast examples/basic.rs
rtc --emit ast --format json examples/basic.rs
```

灰度测试：

```bash
rtc --emit ast examples/basic.rs
```

## Phase 3: 语义分析

目标：AST 到符号表、类型信息和诊断。

支持：

- 作用域树。
- 符号表。
- 未定义变量检查。
- 重复声明检查。
- 基础类型检查。
- 函数调用检查。
- 返回类型检查。
- 条件表达式必须为 bool。

任务：

- 实现 `Scope`。
- 实现 `NameResolver`。
- 实现 `TypeChecker`。
- 设计 `SemanticTrace` JSON。
- 实现 `rtc --check`。
- 实现 `rtc --emit semantic --format json`。

验收：

```bash
cargo test -p rt-semantic
rtc --check examples/basic.rs
rtc --emit semantic --format json examples/basic.rs
```

灰度测试：

```bash
rtc --check examples/basic.rs
```

## Phase 4: LLVM IR 后端

目标：将语义正确的 AST 降低到 LLVM IR，并能通过 LLVM 工具链验证。

支持：

- 常量。
- 算术运算。
- 比较运算。
- 条件跳转。
- basic block。
- 函数调用。
- return。

任务：

- 第一版手写生成 LLVM IR 文本。
- 实现 `rtc -S` 输出 `.ll`。
- 实现 `rtc -c` 调用 LLVM/clang 生成目标文件。
- 实现 `rtc main.rs -o main` 生成可执行文件。
- 在 driver 中调用 LLVM 工具验证 IR。
- 实现 `--emit llvm-ir --format json`。

验收：

```bash
rtc -S examples/basic.rs -o examples/basic.ll
opt-18 -verify examples/basic.ll -disable-output
rtc examples/basic.rs -o examples/basic
./examples/basic
rtc --emit llvm-ir --format json examples/basic.rs
```

灰度测试：

```bash
rtc -S examples/basic.rs -o examples/basic.ll
opt-18 -verify examples/basic.ll -disable-output
```

## Phase 5: 教学 IR 与 CFG 可视化数据

目标：在 LLVM IR 编译通过后，抽取面向教学的简化 IR 和 CFG。

任务：

- 从 LLVM IR module 中提取函数、basic block、instruction。
- 将复杂 LLVM 指令映射为教学 IR 指令。
- 保留 LLVM IR 行号/指令和教学 IR 节点的关联。
- 输出 `--emit teaching-ir`。
- 输出 CFG JSON。

验收：

```bash
rtc --emit teaching-ir examples/basic.rs
rtc --emit teaching-ir --format json examples/basic.rs
rtc --emit all --format json examples/basic.rs
```

## Phase 6: HTML 可视化前端

目标：学生可以在网页中观察编译阶段。

页面布局：

```text
左侧：源码编辑器
右侧：Tabs
  - Tokens
  - AST
  - Semantic
  - IR
  - Diagnostics
```

任务：

- 创建 `web/`。
- 使用 Vite + React + TypeScript。
- 集成 Monaco Editor。
- 实现 Token 表格。
- 实现 AST Tree。
- 实现符号表和作用域树。
- 实现 LLVM IR、教学 IR 和 CFG 展示。
- 实现源码 span 高亮联动。

验收：

```bash
cd web
npm install
npm run dev
```

浏览器中可完成：

- 输入源码。
- 点击 Compile。
- 查看 Token。
- 查看 AST。
- 查看语义信息。
- 查看 LLVM IR。
- 查看教学 IR。
- 查看 CFG。

## Phase 7: Rust 本地 Web Server

目标：前端通过 HTTP 调用编译器。

任务：

- 创建 `rt-server`。
- 使用 Axum。
- 实现 `POST /api/compile`。
- 返回完整 JSON trace。
- 支持错误诊断。

API 示例：

```http
POST /api/compile
Content-Type: application/json

{
  "source": "fn main() { return 1; }",
  "emit": ["tokens", "ast", "semantic", "ir"]
}
```

验收：

```bash
cargo run -p rt-server
```

## Phase 8: 语言特性扩展

按课程需要逐步加入。

建议顺序：

1. `struct`。
2. tuple。
3. array。
4. 简化所有权 move 检查。
5. 简化借用检查。
6. `match`。
7. enum。

每个特性必须包含：

- 正例。
- 反例。
- Token/AST/Semantic/IR 可视化支持。

## Phase 9: Docker 与跨平台

目标：让环境可复现。

任务：

- 添加 Dockerfile。
- 添加 docker-compose。
- 添加 GitHub Actions。
- 加入 Windows frontend-only 构建。
- Linux/Docker 默认启用 LLVM 后端。
- Windows 可先支持前端阶段和可视化，LLVM 后端作为后续兼容任务。

验收：

```bash
docker build -t rtc .
docker run --rm -p 3000:3000 rtc
```

## Phase 10: npm 本地分发

目标：用户通过 npm 安装本地工具。

建议先发布 Linux/Docker 可完整运行版本；Windows 本地版后续补齐 LLVM 打包。

命令：

```bash
npm install -g rust-teaching-compiler
rtc-web
```

架构：

```text
npm package
  -> node wrapper
  -> prebuilt rtc binary
  -> local web server
```

## Phase 11: 云端部署

目标：在线网页实际运行。

架构：

```text
Frontend
  -> API Gateway
  -> Compiler Service
  -> Sandbox Worker
  -> rtc binary
```

安全要求：

- 编译任务超时。
- 内存限制。
- 输出大小限制。
- 容器隔离。
- 禁止任意网络访问。
- 请求限流。

## 待确认问题

这些问题会影响下一步具体设计：

1. 第一版学生代码是否需要支持 `println!`，还是用函数返回值作为运行结果？
2. 第一版是否需要链接 libc，还是只生成无标准库的简单可执行文件？
3. 是否需要支持 `struct` 和简化所有权，作为课程亮点？
4. 项目最终是否要发布 npm 包，还是只作为课程内部平台使用？

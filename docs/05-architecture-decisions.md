# 架构决策记录

## ADR-001: 前端采用 React + TypeScript + Vite

状态：已确认。

原因：

- npm 生态成熟，兼容性强。
- Monaco Editor、Mermaid、表格和树组件集成资料丰富。
- TypeScript 能稳定约束 JSON trace schema。
- Vite 开发体验简单，便于课程项目快速迭代。

约束：

- 不引入过重 UI 框架作为第一版依赖。
- 可视化协议通过 JSON schema 固定，避免前端绑定 Rust 内部 AST。
- 首版组件优先稳定、清晰、可讲解，后续再美化。

## ADR-002: LLVM IR 是主 IR，教学 IR 从 LLVM IR 简化得到

状态：已确认。

原因：

- 项目需要实际可用，不能只做展示用的伪编译。
- LLVM IR 可以被 LLVM 工具链验证、解释执行或生成目标文件。
- 先生成 LLVM IR，再简化为教学 IR，能保证课堂可视化不脱离真实编译过程。

编译链路：

```text
Source
  -> Lexer
  -> Parser
  -> Semantic Analysis
  -> LLVM IR Codegen
  -> LLVM Verification
  -> Teaching IR Projection
  -> HTML Visualization
```

命令目标：

```bash
rtc -S main.rs -o main.ll
rtc -c main.rs -o main.o
rtc main.rs -o main
rtc --emit all --format json main.rs
```

## ADR-003: 项目必须能实际编译和运行受支持子集

状态：已确认。

含义：

- 学生代码不只是被分析，还应能被编译。
- 第一版可只支持小型 Rust 子集。
- 编译产物至少要能生成 LLVM IR。
- Linux 环境中应进一步支持生成可执行文件并运行。

正确性验证策略：

1. 内部单元测试验证 lexer/parser/semantic/codegen。
2. LLVM verifier 验证生成 IR 的结构合法性。
3. 对简单程序运行可执行文件，检查退出码或 stdout。
4. 可选：对受支持子集与 `rustc` 进行行为对比。

## ADR-004: 编译器主体使用 Rust，核心第一版最小化依赖

状态：已确认。

原因：

- 项目是 Rust 教学编译器，使用 Rust 实现有助于学生理解语言自身。
- 编译原理课程需要学生能读懂核心链路，过多第三方库会遮蔽重点。
- 低耦合模块更适合分阶段教学和多人协作。

约束：

- `rt-common`、`rt-lexer`、`rt-parser`、`rt-semantic`、`rt-codegen` 第一版只使用 Rust 标准库。
- CLI 第一版手写参数解析，不使用 `clap`。
- JSON 第一版手写输出，不使用 `serde`。
- LLVM 第一版生成文本 `.ll`，依赖系统 LLVM 工具验证，不直接依赖 `inkwell`。
- 后续引入任何核心依赖都必须补充 ADR。

## ADR-005: 测试采用灰度优先策略

状态：已确认。

含义：

- 每次改动先跑小范围灰度测试，确认最小编译通路仍然可用。
- 灰度通过后，再运行全量单元测试、集成测试和可视化 JSON 测试。
- 对 LLVM 后端，先验证 `.ll` 能通过 `opt-18 -verify`，再测试目标文件和可执行文件。

推荐顺序：

```text
single sample
  -> one stage CLI
  -> LLVM verifier
  -> focused crate tests
  -> full workspace tests
  -> integration examples
```

## ADR-006: 本地环境变量集中放入被忽略目录

状态：已确认。

原因：

- 项目会涉及 LLVM 路径、开发端口、GitHub token 等本地差异配置。
- 这些配置不应该进入版本库。
- 集中放置便于虚拟机、Windows、Docker 分别维护自己的环境。

约定：

- 项目根目录使用 `.local-env/` 存放本地环境变量文件。
- `.local-env/` 必须写入 `.gitignore`。
- 示例文件名：`.local-env/rust-teaching.env`。
- 文档中只描述变量名和用途，不记录真实 token 或密码。
- Git remote URL 不保存 token。

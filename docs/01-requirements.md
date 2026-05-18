# Rust 教学编译器需求文档

## 1. 项目定位

本项目面向“编译原理”课程教学，目标是实现一个 Rust 语言核心子集的教学编译器。项目不追求完整兼容 `rustc`，而是强调编译流程可解释、可观察、可实验。

编译器主体必须使用 Rust 编写。除前端可视化部分外，编译器核心应最小化外部库依赖，保持模块低耦合，便于学生阅读和教师讲解。

学生应能编写一段简化 Rust 代码，通过本项目编译器完成：

1. 词法分析：源码到 Token 流。
2. 语法分析：Token 流到 AST。
3. 语义分析：名称解析、作用域、类型检查、基础所有权检查。
4. LLVM IR 生成：语义正确后直接生成 LLVM IR，并通过 LLVM 工具链验证。
5. 教学 IR 简化：在 LLVM IR 编译通过后，从 LLVM IR/CFG 反向抽取更易读的教学 IR。
6. 可视化观察：在 HTML 前端中逐阶段查看分析结果、错误位置、LLVM IR 和教学 IR。

## 2. 目标用户

- 编译原理课程学生。
- Rust 初学者。
- 教师和助教，用于课堂演示词法、语法、语义和 IR 的关系。
- 项目开发者，用于持续扩展教学实验。

## 3. 非目标

本项目暂不追求：

- 完整实现 Rust 语言。
- 完整兼容 Cargo 包管理、宏系统、trait 系统、生命周期系统。
- 生成生产级优化二进制程序。
- 替代 `rustc`。

但本项目需要做到“实际可用”：学生编写的受支持 Rust 子集代码，应能被本编译器编译为 LLVM IR，并在 Linux 环境中进一步生成可执行文件或通过 LLVM 解释执行。

## 4. 核心语言范围

第一阶段建议支持一个足够教学的 Rust 子集。

### 4.1 基础语法

- `fn main() { ... }`
- 函数定义、函数调用。
- `let` 变量绑定。
- `let mut` 可变绑定。
- 显式类型标注。
- `return` 语句。
- 块表达式。

### 4.2 基础类型

- `i32`
- `bool`
- `char`
- `str` 字符串字面量可先作为特殊类型处理。
- `()` unit 类型。

第二阶段可扩展：

- `i64`
- `f64`
- tuple
- array
- struct

### 4.3 表达式

- 整数字面量。
- 布尔字面量。
- 字符和字符串字面量。
- 标识符表达式。
- 二元运算：`+ - * / % == != < <= > >= && ||`。
- 一元运算：`- !`。
- 赋值表达式：`x = expr`。
- 函数调用：`add(1, 2)`。

### 4.4 控制流

- `if / else`
- `while`
- `loop`
- `break`
- `continue`

第二阶段可扩展：

- `for`
- `match`
- `if let`

### 4.5 语义分析范围

第一阶段应支持：

- 符号表构建。
- 作用域嵌套。
- 重复声明检查。
- 未定义变量检查。
- 类型检查。
- 函数参数数量和类型检查。
- 函数返回类型检查。
- 控制流条件必须为 `bool`。

第二阶段可扩展：

- 简化所有权 move 检查。
- 简化借用检查。
- struct 字段检查。
- enum 和 match 穷尽性检查的教学版本。

## 5. 编译器命令行需求

CLI 参考 gcc 风格，但面向教学增加阶段输出参数。

建议命令名：

```bash
rtc
```

基础命令：

```bash
rtc main.rs
rtc -S main.rs
rtc -c main.rs
rtc -o main main.rs
rtc --check main.rs
```

教学输出：

```bash
rtc --emit tokens main.rs
rtc --emit ast main.rs
rtc --emit semantic main.rs
rtc --emit ir main.rs
rtc --emit all main.rs
```

机器可读输出：

```bash
rtc --emit tokens --format json main.rs
rtc --emit all --format json main.rs
```

建议支持参数：

| 参数 | 含义 |
|---|---|
| `-o <file>` | 指定输出文件 |
| `-S` | 输出 LLVM IR，不生成目标文件或可执行文件 |
| `-c` | 编译到目标文件，不链接 |
| `--check` | 只做语义检查 |
| `--emit tokens` | 输出 Token 流 |
| `--emit ast` | 输出 AST |
| `--emit semantic` | 输出语义信息 |
| `--emit llvm-ir` | 输出 LLVM IR |
| `--emit teaching-ir` | 输出从 LLVM IR/CFG 简化得到的教学 IR |
| `--emit ir` | 默认等价于 `--emit llvm-ir` |
| `--emit all` | 输出所有阶段 |
| `--format text/json` | 输出格式 |
| `--verbose` | 输出阶段日志 |

## 6. HTML 可视化需求

前端页面应提供一个源码编辑区和四个阶段面板。

### 6.1 编辑器

- 支持输入 Rust 子集代码。
- 支持示例代码切换。
- 支持运行编译。
- 支持错误位置高亮。

建议使用 Monaco Editor。

### 6.2 词法分析面板

- 展示 Token 表格。
- 字段包括：kind、lexeme、line、column、span。
- 点击 Token 时高亮源码对应区间。

### 6.3 语法分析面板

- 展示 AST 树。
- 支持折叠/展开。
- 点击 AST 节点时高亮源码区间。
- 显示节点类型、标签、span。

### 6.4 语义分析面板

- 展示符号表。
- 展示作用域树。
- 展示表达式类型标注。
- 展示诊断信息。

### 6.5 IR 面板

- 默认展示 LLVM IR 文本。
- 展示从 LLVM IR/CFG 简化得到的教学 IR。
- 展示 basic block 控制流图。
- 支持 LLVM IR 和教学 IR 的 basic block 联动。

## 7. JSON 输出协议需求

为了支持 HTML 可视化，编译器需要输出稳定 JSON。

统一结构：

```json
{
  "version": "0.1.0",
  "source_name": "main.rs",
  "stages": {
    "lexer": {},
    "parser": {},
    "semantic": {},
    "llvm_ir": {},
    "teaching_ir": {}
  },
  "diagnostics": []
}
```

诊断结构：

```json
{
  "level": "error",
  "code": "E0001",
  "message": "undefined variable `x`",
  "span": {
    "start": 10,
    "end": 11,
    "line": 1,
    "column": 11
  }
}
```

Token 结构：

```json
{
  "kind": "Ident",
  "lexeme": "main",
  "span": {
    "start": 3,
    "end": 7,
    "line": 1,
    "column": 4
  }
}
```

AST 节点结构：

```json
{
  "id": 1,
  "kind": "Function",
  "label": "main",
  "span": {
    "start": 0,
    "end": 20,
    "line": 1,
    "column": 1
  },
  "children": []
}
```

## 8. 质量要求

课程项目的质量重点：

- 能稳定展示编译阶段。
- 错误信息对学生友好。
- 代码结构清晰，便于讲解。
- 编译器核心模块低耦合。
- 外部库依赖尽量少，必要依赖必须有明确理由。
- 测试覆盖基础语法、错误用例和可视化 JSON 输出。
- 每个阶段都能单独运行。
- 测试时先进行灰度测试，验证最小通路，再进行全量测试。

第一阶段验收标准：

- 能在 Linux 本地运行 `cargo test`。
- 能运行 `rtc --emit tokens --format json examples/basic.rs`。
- 能运行 `rtc --emit ast --format json examples/basic.rs`。
- 能运行 `rtc --check examples/basic.rs`。
- 能运行 `rtc -S examples/basic.rs -o examples/basic.ll`。
- 能用 LLVM 工具链验证生成的 `.ll`。
- HTML 前端能展示 Token 和 AST。

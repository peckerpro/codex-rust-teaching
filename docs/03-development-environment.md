# 当前开发环境与运行环境

## 1. 当前实际环境

当前开发资源包括：

### Windows 主机

- 用途：Codex 控制端、文档编辑、跨平台验证、前端开发。
- 工作目录：`D:\codex-project\rust-teaching`
- Shell：PowerShell。

### Ubuntu 虚拟机

- 用途：Linux-first 开发和测试环境。
- 访问方式：SSH。
- 用户：`pecker`
- 项目目录：`/home/pecker/codex-project/rust-teaching`
- 当前已确认：

```text
OS: Ubuntu 24.04 系列
Kernel: Linux 6.17.0-23-generic x86_64
LLVM: /usr/bin/llvm-config-18
```

## 2. Ubuntu 开发环境建议

基础工具：

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  git \
  pkg-config \
  libssl-dev \
  clang \
  lld \
  llvm-18 \
  llvm-18-dev \
  llvm-18-tools \
  libclang-18-dev
```

Rust 工具链：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup component add rustfmt clippy rust-analyzer
```

LLVM 环境变量：

```bash
export LLVM_SYS_180_PREFIX="$(llvm-config-18 --prefix)"
```

建议写入 `~/.bashrc`：

```bash
echo 'export LLVM_SYS_180_PREFIX="$(llvm-config-18 --prefix)"' >> ~/.bashrc
```

Node.js 用于前端：

```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt install -y nodejs
node --version
npm --version
```

LLVM 验证工具：

```bash
llvm-as-18 --version
opt-18 --version
llc-18 --version
clang-18 --version
```

如果 `clang-18` 不存在，可以先使用系统 `clang`，但 CI 和 Docker 中应固定到 LLVM 18。

## 3. 开发命令约定

### 本地环境变量

项目根目录下使用 `.local-env/` 保存本机或虚拟机专属环境变量，例如：

```text
.local-env/rust-teaching.env
```

该目录必须被 git 忽略，不能提交到仓库。里面可以保存：

- `LLVM_SYS_180_PREFIX`
- 本地开发端口
- 本地服务地址
- GitHub token 等临时凭据

使用方式：

```bash
source .local-env/rust-teaching.env
```

注意：任何 token 或密码都不能写入 README、docs、源码或 git remote URL。

Rust 编译器核心：

```bash
cargo build
cargo test
cargo fmt --check
cargo clippy --workspace --all-targets
```

CLI 运行：

```bash
cargo run -p rt-cli -- examples/basic.rs
cargo run -p rt-cli -- -S examples/basic.rs -o examples/basic.ll
cargo run -p rt-cli -- --emit tokens --format json examples/basic.rs
cargo run -p rt-cli -- --emit ast --format json examples/basic.rs
cargo run -p rt-cli -- --check examples/basic.rs
```

LLVM IR 验证：

```bash
llvm-as-18 examples/basic.ll -o /tmp/basic.bc
opt-18 -verify examples/basic.ll -disable-output
lli-18 examples/basic.ll
```

前端开发：

```bash
cd web
npm install
npm run dev
```

## 4. Docker 运行环境规划

Docker 用于提供一致的 Linux 环境。

建议提供：

```text
Dockerfile
docker-compose.yml
```

Dockerfile 应包含：

- Rust stable。
- LLVM 18。
- Node.js 22。
- 项目依赖。

建议命令：

```bash
docker build -t rust-teaching-compiler .
docker run --rm -it -p 3000:3000 rust-teaching-compiler
```

## 5. Windows 支持规划

Windows 支持不是第一优先级，但设计时应避免制造明显障碍。

需要注意：

- 不在核心代码里硬编码 `/tmp`。
- 使用 `std::env::temp_dir()`。
- 不依赖 bash 作为唯一测试入口。
- LLVM 后端做成可选 feature。
- `--emit tokens/ast/semantic` 不应依赖 LLVM。

## 6. CI 规划

后续使用 GitHub Actions 做自动验证。

推荐 matrix：

```text
ubuntu-latest: 完整测试，包含 LLVM 后端
windows-latest: frontend-only 测试
macos-latest: frontend-only 测试
```

第一版 CI 只需要：

```bash
cargo fmt --check
cargo test --workspace
```

后续再加入：

```bash
cargo clippy --workspace --all-targets -- -D warnings
npm test
npm run build
```

# SSH Split Pane

[English](README.md) | [中文](README-zh.md)

Windows/Linux 桌面工具：通过 SSH 做**远端双栏**文件浏览。
在两个面板间拖拽条目，在**同一台主机**上执行远端 `cp` 或 `mv`（不经过本机传文件——不是 SFTP 同步客户端）。

**一句话：** 同机双栏，拖拽即远端 `cp`/`mv`。

## 下载

预编译包（发布后）见 [GitHub Releases](../../releases)，当前标签 **v0.1.2**。

| 平台 | 典型产物名 |
|------|------------|
| Linux x86_64 | `ssh_pane-v0.1.2-linux-x86_64.tar.gz` |
| Windows x86_64 | `ssh_pane-v0.1.2-windows-x86_64.zip` |

若尚未挂载 Release 资源，可自行编译（见下文），或执行：

```bash
./scripts/package-release.sh v0.1.2
```

变更说明见 [RELEASE_NOTES.md](RELEASE_NOTES.md)。

## 功能

- 密码或私钥登录（登录页 **Auth** 切换）
- **View → Theme → Dark / Light** 切换主题；偏好会持久化
- **File / View / Help** 菜单栏
- **CP / MV** 模式切换（工具栏高亮按钮）
- 左右分栏，各自独立远端路径
- 从一侧拖到另一侧，在服务器上复制或移动
- **Ctrl+点击** 多选；右键 → **Move N items to other pane**
- 每栏 **Filter** 过滤框（不区分大小写的名称子串）
- 每栏 **Preview ON/OFF**，远端图片预览（png/jpg/gif/webp/bmp）
- 新建文件夹（面板 **+ Folder** 或 File 菜单）
- 目标已存在同名项时弹出覆盖确认
- 通过 `log` / `env_logger` 做结构化日志

## 已知限制（v0.1）

- 暂不支持带 **passphrase** 的私钥
- 仅支持**同机**远端 `cp`/`mv`，不做跨主机或本机↔远端同步
- **未校验** `known_hosts` 主机密钥（在不可信网络上有中间人风险）
- **密码不落盘**；可选仅记住 host / 用户名
- 尚未提供 macOS 构建说明

## 安全说明

- 不要把凭据提交进仓库；真实 SSH 测试只用环境变量
- 能用私钥时优先用私钥
- 早期版本更适合可信网络 / 可信主机

漏洞报告见 [SECURITY.md](SECURITY.md)。

## 环境要求

- Rust 1.74+（stable）
- 使用带 `vendored-openssl` 的 `ssh2` 时需要 C 工具链（Debian/Ubuntu 用 `build-essential`；Windows 用 MSVC Build Tools）

### Windows（MSVC）

1. 安装 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)，勾选 “Desktop development with C++”
2. 为 `x86_64-pc-windows-msvc` 安装 [Rust](https://rustup.rs/)
3. 在本目录执行：

```bat
cargo build --release
```

可执行文件：`target\release\ssh_pane.exe`

### Linux / WSL（GUI 需 WSLg）

```bash
sudo apt install build-essential pkg-config libssl-dev
# 可选：更快的链接/缓存（或使用 ~/.local/bin 里预装的 mold/sccache）
#   sudo apt install mold clang sccache
# WSL2 下跑 GUI：开启 WSLg，或直接用 Windows 编好的 exe

# 推荐用项目包装脚本（固定 ./target，若有 mold/sccache 会自动启用）
./scripts/cargo.sh build          # debug，依赖 opt-level 2
./scripts/cargo.sh build --release  # thin LTO（比 fat LTO 更快）
./scripts/cargo.sh build --profile dist  # fat LTO，适合发布包

./target/release/ssh_pane
```

**编译速度说明**

- 使用 `./scripts/cargo.sh …`，让 `CARGO_TARGET_DIR` 固定在 `./target`（避免沙箱/缓存把产物写到别处）。
- Dev profile 会对依赖开 `opt-level = 2`，egui 更流畅，自己的 crate 仍能较快重编。
- 默认 `--release` 使用 **thin LTO**；需要极致体积/速度时用 `--profile dist`。
- `PATH` 里有 `mold` 和 `sccache` 时，包装脚本会自动打开（对 vendored OpenSSL 帮助很大）。
- 更详细的编译耗时说明见 [docs/BUILD.md](docs/BUILD.md)。

从 Linux 交叉编译 Windows（可选）：

```bash
rustup target add x86_64-pc-windows-gnu
# 需要 mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
```

## 使用方法

1. 启动应用，填写 host、port、user；在 **Auth** 中选择 **Password** 或 **Private key**。在凭据输入框按 **Enter** 即可连接。
2. 连接后，在工具栏选择 **CP** 或 **MV**。
3. 在各栏导航（双击文件夹、**Up**、路径 + **Go**）。
4. 将文件或文件夹从一栏拖到另一栏列表上。

操作在远端以 shell 命令执行：

- 复制：`cp -a -- <src> <dst_dir>/`
- 移动：`mv -- <src> <dst_dir>/`

### 日志

```bash
# 默认：ssh_pane=info
RUST_LOG=ssh_pane=debug ./target/release/ssh_pane
```

## 测试

```bash
# 单元测试（crate 内）+ 功能测试（tests/）
cargo test

# 仅功能测试套件
cargo test --test functional_ops --test functional_pathutil --test functional_worker

# 可选：真实 SSH 功能测试（默认 ignored）
SSH_PANE_TEST_HOST=127.0.0.1 \
SSH_PANE_TEST_USER=me \
SSH_PANE_TEST_PASSWORD=secret \
cargo test --test functional_live_ssh -- --ignored --nocapture
```

## 项目结构

```
src/
  lib.rs         # 库入口（pathutil / ops / ssh_worker / logging）
  main.rs        # eframe 入口 + 日志初始化
  app.rs         # UI 状态机
  logging.rs     # env_logger 配置
  ops.rs         # 纯远端命令规划（可测）
  pathutil.rs    # 远端路径 + shell 引号
  ssh_worker.rs  # 后台 SSH/SFTP 线程
  ui/            # 连接、工具栏、面板、菜单等控件
tests/
  functional_ops.rs       # CP/MV/mkdir 规划场景
  functional_pathutil.rs  # 导航 / 引号场景
  functional_worker.rs    # 无会话的 worker 消息
  functional_live_ssh.rs  # 可选真实 SSH（--ignored）
```

## 许可证

[MIT](LICENSE) © 2026 phillee

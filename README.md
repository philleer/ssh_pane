# SSH Split Pane

Windows/Linux desktop tool for **remote dual-pane** file browsing over SSH.
Drag items between panes to run remote `cp` or `mv` on the same host (no local file transfer).

## Features

- Connect via password or private key (no passphrase in v0.1)
- **File / View / Help** menu bar
- **CP / MV** mode toggle (highlighted toolbar buttons)
- Split left/right panes, each with its own remote path
- Drag from one pane to the other to copy or move on the server
- Per-pane **Preview ON/OFF** toggle for remote image preview (png/jpg/gif/webp/bmp)
- Create new folders (pane **+ Folder** or File menu)
- Overwrite confirmation when the destination name already exists
- Structured logging via `log` / `env_logger`

## Requirements

- Rust 1.74+ (stable)
- For `ssh2` with `vendored-openssl`: a C toolchain (`build-essential` on Debian/Ubuntu; MSVC Build Tools on Windows)

### Windows (MSVC)

1. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with “Desktop development with C++”
2. Install [Rust](https://rustup.rs/) for `x86_64-pc-windows-msvc`
3. From this directory:

```bat
cargo build --release
```

Binary: `target\release\ssh_split_pane.exe`

### Linux / WSL (WSLg for GUI)

```bash
sudo apt install build-essential pkg-config libssl-dev
# Optional: faster link/cache (or use the prebuilt mold/sccache in ~/.local/bin)
#   sudo apt install mold clang sccache
# For GUI under WSL2, enable WSLg or use a Windows-built exe

# Prefer the project wrapper (pins ./target, enables mold/sccache if present)
./scripts/cargo.sh build          # debug, deps at opt-level 2
./scripts/cargo.sh build --release  # thin LTO (faster than fat LTO)
./scripts/cargo.sh build --profile dist  # fat LTO for shipping

./target/release/ssh_split_pane
```

**Build speed notes**

- Use `./scripts/cargo.sh …` so `CARGO_TARGET_DIR` stays at `./target` (avoids sandbox/cache redirects writing elsewhere).
- Dev profile optimizes dependencies (`opt-level = 2`) for snappier egui while your crates stay quick to recompile.
- Default `--release` uses **thin LTO**; use `--profile dist` when you want maximum size/speed optimization.
- With `mold` and `sccache` on `PATH`, the wrapper turns them on automatically (helps a lot with vendored OpenSSL).

Cross-compile for Windows from Linux (optional):

```bash
rustup target add x86_64-pc-windows-gnu
# needs mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
```

## Usage

1. Launch the app and enter host, port, user, and password (or pick a private key).
2. After connect, choose **CP** or **MV** in the toolbar.
3. Navigate each pane (double-click folders, **Up**, path + **Go**).
4. Drag a file or folder from one pane onto the other pane’s list.

Operations run as remote shell commands:

- Copy: `cp -a -- <src> <dst_dir>/`
- Move: `mv -- <src> <dst_dir>/`

### Logging

```bash
# default: ssh_split_pane=info
RUST_LOG=ssh_split_pane=debug ./target/release/ssh_split_pane
```

## Tests

```bash
# Unit tests (in-crate) + functional tests (tests/)
cargo test

# Functional suites only
cargo test --test functional_ops --test functional_pathutil --test functional_worker

# Optional live SSH functional test (ignored by default)
SSH_SPLIT_PANE_TEST_HOST=127.0.0.1 \
SSH_SPLIT_PANE_TEST_USER=me \
SSH_SPLIT_PANE_TEST_PASSWORD=secret \
cargo test --test functional_live_ssh -- --ignored --nocapture
```

## Project layout

```
src/
  lib.rs         # library root (pathutil / ops / ssh_worker / logging)
  main.rs        # eframe entry + logger init
  app.rs         # UI state machine
  logging.rs     # env_logger setup
  ops.rs         # pure remote command planning (testable)
  pathutil.rs    # remote paths + shell quoting
  ssh_worker.rs  # background SSH/SFTP thread
  ui/            # connect, toolbar, pane, menu widgets
tests/
  functional_ops.rs       # CP/MV/mkdir planning scenarios
  functional_pathutil.rs  # navigation / quoting scenarios
  functional_worker.rs    # worker messaging without a session
  functional_live_ssh.rs  # optional live SSH (--ignored)
```

## License

MIT

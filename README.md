# SSH Split Pane

Windows/Linux desktop tool for **remote dual-pane** file browsing over SSH.
Drag items between panes to run remote `cp` or `mv` on the same host (no local file transfer).

## Features

- Connect via password or private key (no passphrase in v0.1)
- **File / View / Help** menu bar
- **CP / MV** mode toggle (highlighted toolbar buttons)
- Split left/right panes, each with its own remote path
- Drag from one pane to the other to copy or move on the server
- Create new folders (pane **+ Folder** or File menu)
- Overwrite confirmation when the destination name already exists

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
# For GUI under WSL2, enable WSLg or use a Windows-built exe
cargo build --release
./target/release/ssh_split_pane
```

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

## Project layout

```
src/
  main.rs        # eframe entry
  app.rs         # UI state machine
  ssh_worker.rs  # background SSH/SFTP thread
  pathutil.rs    # remote paths + shell quoting
  ui/            # connect, toolbar, pane widgets
```

## License

MIT

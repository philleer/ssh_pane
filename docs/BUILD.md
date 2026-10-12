# Build time analysis

## Why first / release builds feel slow

| Cost center | Why it hurts | Mitigation already in repo |
|-------------|--------------|----------------------------|
| **`ssh2` + `vendored-openssl`** | Compiles OpenSSL from C on first build; minutes on cold cache | `sccache` via `./scripts/cargo.sh`; keep `CARGO_TARGET_DIR=./target` |
| **`eframe` / `egui` / wgpu stack** | Large Rust dependency graph | Dev profile uses incremental; mold linker on Linux |
| **`image` codecs** | jpeg/png/gif/webp/bmp each add compile work | Features already trimmed (`default-features = false`) |
| **`[profile.dev.package."*"] opt-level = 2`** | Dependencies compiled optimized → **first** `cargo build` slower, day-to-day UI snappier | Intentional tradeoff; our crates stay `opt-level = 0` |
| **`[profile.release] lto = "thin"`** | Link-time optimization adds link time | Prefer `./scripts/cargo.sh build --release` over `--profile dist` while iterating |
| **`[profile.dist]` fat LTO + `codegen-units = 1`** | Slowest link; for shipping only | Use only for final packages (`./scripts/package-release.sh`) |

## Expected rough times (cold vs warm)

- **Cold debug** (no sccache): often dominated by OpenSSL + egui (several minutes).
- **Warm incremental** (edit `src/ui/*.rs`): usually seconds with incremental + mold.
- **Release / dist**: link phase dominates; `dist` ≫ `release` (thin LTO).

## Recommended commands

```bash
./scripts/cargo.sh build              # daily
./scripts/cargo.sh build --release    # try binaries
./scripts/cargo.sh build --profile dist   # ship only
sccache -s                            # see cache hit rate if installed
```

## If builds are still slow

1. Confirm you use `./scripts/cargo.sh` (mold + sccache + fixed `./target`).
2. Avoid `cargo clean` unless necessary (wipes OpenSSL/egui artifacts).
3. Do not use `--profile dist` for local UI iteration.
4. Optional: `cargo build -j$(nproc)` is default; disk on WSL/antivirus can bottleneck more than CPU.

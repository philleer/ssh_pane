# Build time analysis

FAQ-style write-up of the release / incremental question: [FAQs.md](FAQs.md).

## Why your ~35s “incremental release” is slow

Progress stuck on `380/381: ssh_pane(bin)` means **dependencies are fine**; almost all time is **recompiling this crate + linking**.

For `cargo build --release`:

1. Cargo **turns off incremental** on the `release` profile by default.
2. This project enables **`lto = "thin"`**, so every binary rebuild re-runs thin LTO over egui/ssh2/etc.
3. Result: a one-line UI edit still costs ~30s+ even though only `ssh_pane` shows as compiling.

Full cold release (~1m) is dominated by deps once; warm “incremental release” is dominated by **LTO link**, not by missing sccache/mold.

## What to use when

| Goal | Command | Expectation |
|------|---------|-------------|
| Daily UI work | `./scripts/cargo.sh build` | Seconds after warm cache (deps already `opt-level=2`) |
| Fast optimized binary | `./scripts/cargo.sh build --profile rel` | Near-release speed, **no LTO**, incremental on |
| Try / share release binary | `./scripts/cargo.sh build --release` | ~1m cold / ~30s+ per edit (thin LTO) |
| Ship smallest/fastest | `./scripts/cargo.sh build --profile dist` | Slowest link; packaging only |

Aliases (via `.cargo/config.toml`): `cargo rel`, `cargo br`, `cargo dist` (prefer `./scripts/cargo.sh …`).

## Cost centers (cold builds)

| Cost center | Why it hurts | Mitigation |
|-------------|--------------|------------|
| **`ssh2` + `vendored-openssl`** | Compiles OpenSSL from C | `sccache` + keep `./target` |
| **`eframe` / `egui`** | Huge dep graph | Don’t `cargo clean` casually |
| **`image` codecs** | Extra crates | Features already trimmed |
| **`dev.package.*` opt-level=2** | First debug slower, runtime snappier | Intentional |
| **`release` thin LTO** | Every bin link expensive | Use `rel` or `dev` to iterate |
| **`dist` fat LTO** | Slowest | Ship only |

## Recommended commands

```bash
./scripts/cargo.sh build                 # daily (debug + optimized deps)
./scripts/cargo.sh build --profile rel   # optimized, fast incremental
./scripts/cargo.sh build --release       # thin LTO when you need it
./scripts/cargo.sh build --profile dist  # final package
sccache -s                               # cache hit rate
```

## If `rel` / `dev` is still slow

1. Use `./scripts/cargo.sh` (mold + sccache + fixed `./target`).
2. Avoid `cargo clean`.
3. WSL disk / antivirus can dominate link time more than CPU.
4. Do not iterate on `--release` or `--profile dist`.

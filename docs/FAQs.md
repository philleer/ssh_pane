# FAQs

## Why is “incremental” `cargo build --release` still ~30s?

**Short answer:** `release` turns **incremental compilation off by default**, and this repo’s `release` profile also uses **thin LTO**. After a small edit you mostly pay for a full thin-LTO **link** of `ssh_pane`, not for rebuilding egui/OpenSSL again.

### What the progress bar means

```
Building [=======================> ] 380/381: ssh_pane(bin)
Finished `release` profile … in ~33s
```

- **380/381** → dependencies are already built; only this package is left.
- Almost all of those ~30s is **recompile this crate + thin-LTO link** (eframe/ssh2 pulled into the final binary).
- sccache / mold help cold and dep builds; they do **not** remove the LTO link tax on every release rebuild.

### Cargo defaults (release)

| Setting | Default on `release` | Effect here |
|---------|----------------------|-------------|
| `incremental` | **off** | No incremental object reuse for your crate |
| `lto` (this repo) | `"thin"` | Every binary rebuild re-runs thin LTO |
| Observed | — | Full release ~1m; “touch one file + release” ~30s+ |

### What to do instead

| Goal | Command |
|------|---------|
| Daily UI iteration | `./scripts/cargo.sh build` |
| Optimized + fast incremental | `./scripts/cargo.sh build --profile rel` |
| Real thin-LTO release | `./scripts/cargo.sh build --release` |
| Ship (fat LTO) | `./scripts/cargo.sh build --profile dist` |

`rel` keeps `opt-level = 3`, sets `lto = false` and `incremental = true`. In practice, a warm touch rebuild can drop from ~30s (`--release`) to ~1–2s (`--profile rel`). Binary: `target/rel/ssh_pane`.

### Related

More build-cost tables and commands: [BUILD.md](BUILD.md).

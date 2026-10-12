# FAQs

## How to add a new entry

1. Append the next number at the **end** of this file (`Q2` / `A2`, then `Q3` / `A3`, …). Do not renumber old items.
2. Use this template:

```markdown
## QN. <short question>

### AN.

<analysis — why it happens>

**Solution**

<commands / workaround>

- Recorded: YYYY-MM-DD (+08:00)
- Environment: ubuntu-XX.YY (…), …
```

3. Keep the file-level header (date / OS / toolchain) as the **baseline** machine for Q1. Per-question footnotes are optional when the env differs.
4. Prefer one concrete problem per Q; link to [BUILD.md](BUILD.md) or README instead of duplicating long tables.

Baseline (Q1): **2026-10-12** (+08:00) · **ubuntu-26.04** (WSL2), Linux `6.18.33.2-microsoft-standard-WSL2` x86_64 · rustc/cargo **1.98.1**, mold **2.40.1**

---

## Q1. Why is “incremental” `cargo build --release` still ~30s?

### A1.

`release` turns **incremental compilation off by default**, and this repo’s `release` profile also uses **thin LTO**. After a small edit you mostly pay for a full thin-LTO **link** of `ssh_pane`, not for rebuilding egui/OpenSSL again.

**Progress bar meaning**

```
Building [=======================> ] 380/381: ssh_pane(bin)
Finished `release` profile … in ~33s
```

- **380/381** → dependencies are already built; only this package is left.
- Almost all of those ~30s is **recompile this crate + thin-LTO link** (eframe/ssh2 pulled into the final binary).
- sccache / mold help cold and dep builds; they do **not** remove the LTO link tax on every release rebuild.

**Cargo / profile facts (release)**

| Setting | Default on `release` | Effect here |
|---------|----------------------|-------------|
| `incremental` | **off** | No incremental object reuse for your crate |
| `lto` (this repo) | `"thin"` | Every binary rebuild re-runs thin LTO |
| Observed (this machine) | — | Full release ~1m 12s; warm “edit + release” ~33–35s |

**Solution**

| Goal | Command |
|------|---------|
| Daily UI iteration | `./scripts/cargo.sh build` |
| Optimized + fast incremental | `./scripts/cargo.sh build --profile rel` |
| Real thin-LTO release | `./scripts/cargo.sh build --release` |
| Ship (fat LTO) | `./scripts/cargo.sh build --profile dist` |

`rel` keeps `opt-level = 3`, sets `lto = false` and `incremental = true`. On this environment a warm touch rebuild dropped from ~30s (`--release`) to ~1.5s (`--profile rel`). Binary: `target/rel/ssh_pane`.

More build-cost tables: [BUILD.md](BUILD.md).

---

## Q2. What is the difference between `target/rel` and `target/release`? Why use `rel`?

### A2.

They are **two different Cargo profiles**, writing into different directories. Nothing replaced official `release`; `rel` was added for **day-to-day iteration**.

| | `target/release` | `target/rel` |
|--|------------------|--------------|
| Command | `./scripts/cargo.sh build --release` | `./scripts/cargo.sh build --profile rel` |
| Optimization | `opt-level = 3` | `opt-level = 3` |
| LTO | **thin** (this repo) | **off** |
| Incremental | **off** (Cargo default for `release`) | **on** |
| Warm rebuild after a UI edit | often **~30s+** (relink) | about **1–2s** |
| Use for | try / share a near-shipping binary | daily UI work |

**Why `rel` exists:** Q1 — warm `--release` rebuilds still pay thin-LTO link time. `rel` keeps release-like speed at runtime for local use, without that link tax.

**What did not change:** shipping / GitHub Release packages still use `--release` or `--profile dist` (`target/release` / `target/dist`). Prefer `target/rel/ssh_pane` only while developing.

- Recorded: 2026-10-12 (+08:00)
- Environment: same baseline as Q1 (ubuntu-26.04 WSL2)

# Release Notes

## v0.1.4

- Connect UI: larger form fonts; slogan below field labels; auth switch on the same row (**Use key** / **Password**)
- Help menu polish; About dialog (movable, bordered, version from crate)
- Refresh connect screenshot

## v0.1.3

- **Preview** button label only (color shows on/off); item count sits after the filter box
- Add `--profile rel` for fast optimized incremental builds (see `docs/BUILD.md`)

## v0.1.2

- Rename crate / binary / config / env vars from `ssh_split_pane` to `ssh_pane` (UI display name remains **SSH Split Pane**)
- Settings migrate from the old `ssh_split_pane` config path when present
- **View → Theme → Dark / Light** (persisted); theme applies on the connect screen after startup
- Drop Left/Right pane titles and similar chrome on the browser UI
- Startup / connect-failure logging (`RUST_LOG=ssh_pane=debug` for more detail)
- Repo cleanup; build-time notes in `docs/BUILD.md`

## v0.1.1

- Login **Auth** toggle: default **Password**, or switch to **Private key**
- Press **Enter** in the password / key path field to connect
- Add Chinese docs `README-zh.md`
- Open-source packaging: `LICENSE`, `SECURITY.md`, issue templates, download / limitations sections

## v0.1.0

- First usable release: SSH remote dual-pane browse; drag to run remote `cp` / `mv`
- Password or private-key login (no passphrase yet)
- Multi-select batch move, name filter, image preview, mkdir, overwrite confirm

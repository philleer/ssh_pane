# Release Notes

## v0.1.2

- Rename crate / binary / config / env vars from `ssh_split_pane` to `ssh_pane` (UI display name remains **SSH Split Pane**)
- Settings still load from the old `ssh_split_pane` config path if present

## v0.1.1

- Login **Auth** toggle: default **Password**, or switch to **Private key**
- Press **Enter** in the password / key path field to connect
- Add Chinese docs `README-zh.md`
- Open-source packaging: `LICENSE`, `SECURITY.md`, issue templates, download / limitations sections

## v0.1.0

- First usable release: SSH remote dual-pane browse; drag to run remote `cp` / `mv`
- Password or private-key login (no passphrase yet)
- Multi-select batch move, name filter, image preview, mkdir, overwrite confirm

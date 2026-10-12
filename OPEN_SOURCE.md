# Open-source publish checklist

Use this when pushing the repo public for the first time.

## Done in-tree

- [x] `LICENSE` (MIT) with copyright holder
- [x] `Cargo.toml` `license` / `authors` / `keywords`
- [x] Bilingual README + known limitations + security notes
- [x] `SECURITY.md`
- [x] Issue templates under `.github/ISSUE_TEMPLATE/`
- [x] `RELEASE_NOTES.md` + tag `v0.1.1`
- [x] `scripts/package-release.sh` for local artifacts

## On GitHub (after `git remote` + push)

1. Create the repository (public), push `feat-feedback` or `main`, push tags:
   ```bash
   git remote add origin git@github.com:<you>/ssh_split_pane.git
   git push -u origin HEAD
   git push origin v0.1.1
   ```
2. **Topics** (Settings → or repo sidebar):  
   `ssh` `sftp` `file-manager` `dual-pane` `rust` `egui` `remote`
3. Replace `OWNER` in `.github/ISSUE_TEMPLATE/config.yml` with your GitHub user/org.
4. Set `repository` / `homepage` in `Cargo.toml` once the URL is final.
5. **Release**: build packages, then publish:
   ```bash
   ./scripts/package-release.sh v0.1.1
   gh auth login   # once
   gh release create v0.1.1 dist/* --notes-file RELEASE_NOTES.md
   ```
6. Add real screenshots under `docs/screenshots/` and uncomment the image links in README.
7. Optional: crates.io publish (`cargo publish`) after verifying the crate name is free.

## Name / branding reminders

- Display name: **SSH Split Pane**
- Binary / crate: `ssh_split_pane`
- One-liner: *Same-host dual pane — drag to cp/mv over SSH.*

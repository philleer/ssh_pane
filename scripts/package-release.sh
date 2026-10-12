#!/usr/bin/env bash
# Build and pack release artifacts into ./dist for GitHub Releases.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
  VERSION="$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/')"
  VERSION="v${VERSION}"
fi
VERSION="${VERSION#v}"
TAG="v${VERSION}"

DIST="$ROOT/dist"
rm -rf "$DIST"
mkdir -p "$DIST"

echo "==> Building Linux release ($TAG)"
if [[ -x ./scripts/cargo.sh ]]; then
  ./scripts/cargo.sh build --profile dist
else
  cargo build --profile dist
fi

BIN="$ROOT/target/dist/ssh_split_pane"
if [[ ! -f "$BIN" ]]; then
  BIN="$ROOT/target/release/ssh_split_pane"
fi
if [[ ! -f "$BIN" ]]; then
  echo "error: binary not found" >&2
  exit 1
fi

STAGE="$DIST/stage-linux"
mkdir -p "$STAGE"
cp "$BIN" "$STAGE/ssh_split_pane"
cp LICENSE README.md README-zh.md RELEASE_NOTES.md "$STAGE/"
tar -C "$STAGE" -czf "$DIST/ssh_split_pane-${TAG}-linux-x86_64.tar.gz" .
rm -rf "$STAGE"

echo "==> Wrote $DIST/ssh_split_pane-${TAG}-linux-x86_64.tar.gz"
echo
echo "Windows: build on MSVC or cross-compile, then zip ssh_split_pane.exe + LICENSE + READMEs as:"
echo "  ssh_split_pane-${TAG}-windows-x86_64.zip"
echo
echo "Publish (after gh auth login + remote):"
echo "  gh release create ${TAG} dist/* --notes-file RELEASE_NOTES.md"

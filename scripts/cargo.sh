#!/usr/bin/env bash
# Project-local cargo wrapper: keeps artifacts in ./target and enables
# mold / sccache when available. Use instead of bare `cargo` in this repo.
#
# Examples:
#   ./scripts/cargo.sh build
#   ./scripts/cargo.sh build --release
#   ./scripts/cargo.sh test
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Always write into the workspace target dir (avoids Cursor sandbox redirects).
export CARGO_TARGET_DIR="${ROOT}/target"

if command -v sccache >/dev/null 2>&1; then
  export RUSTC_WRAPPER="${RUSTC_WRAPPER:-sccache}"
fi

# Prefer mold for faster linking on Linux (gcc/clang -fuse-ld=mold).
if [[ "$(uname -s)" == "Linux" ]] && command -v mold >/dev/null 2>&1; then
  extra="-C link-arg=-fuse-ld=mold"
  if [[ -n "${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS:-}" ]]; then
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS} ${extra}"
  else
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="${extra}"
  fi
fi

exec cargo "$@"

#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:?Usage: $0 <target-triple>}"
VERSION="${TMJ_VERSION:-0.0.0}"

rustup target add "$TARGET" --no-self-update 2>/dev/null || true

cargo build --release --locked --target "$TARGET" -p tmj_terminal -p tmj_wgpu

STAGING="staging/${TARGET}"
mkdir -p "$STAGING"
cp "target/${TARGET}/release/tmj_terminal" "$STAGING/"
cp "target/${TARGET}/release/tmj_wgpu" "$STAGING/"
cp README.md "$STAGING/"
cp LICENSE "$STAGING/"

echo "Staged to ${STAGING}/"

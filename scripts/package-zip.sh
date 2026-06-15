#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:?Usage: $0 <target-triple>}"
VERSION="${2:?Usage: $0 <target-triple> <version>}"
VARIANT="${3:?Usage: $0 <target-triple> <version> <tmj|tmj-wgpu>}"

ARTIFACT_NAME="${VARIANT}-${TARGET}-v${VERSION}"
STAGING="staging/${TARGET}"

case "$VARIANT" in
    tmj)       BINARY="tmj_terminal" ;;
    tmj-wgpu)  BINARY="tmj_wgpu" ;;
    *) echo "Invalid variant: $VARIANT (expected tmj or tmj-wgpu)"; exit 1 ;;
esac

ZIP_DIR="target/zips/${VARIANT}"
rm -rf "$ZIP_DIR"
mkdir -p "$ZIP_DIR"

cp "${STAGING}/${BINARY}" "$ZIP_DIR/"
cp "${STAGING}/README.md" "$ZIP_DIR/"
cp "${STAGING}/LICENSE"     "$ZIP_DIR/"
cp doc/logo.png             "$ZIP_DIR/logo.png"

mkdir -p target/artifacts
cd "$ZIP_DIR"
zip -r "../../artifacts/${ARTIFACT_NAME}.zip" .
cd - > /dev/null

rm -rf "target/zips"
echo "Built target/artifacts/${ARTIFACT_NAME}.zip"

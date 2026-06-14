#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:?Usage: $0 <target-triple>}"
VERSION="${2:?Usage: $0 <target-triple> <version>}"
VARIANT="${3:?Usage: $0 <target-triple> <version> <tmj|tmj-wgpu>}"
VERSION="${VERSION#v}"

case "$TARGET" in
    x86_64-unknown-linux-gnu)   DEB_ARCH="amd64" ;;
    aarch64-unknown-linux-gnu)  DEB_ARCH="arm64" ;;
    *) echo "Unsupported target for .deb: $TARGET"; exit 1 ;;
esac

case "$VARIANT" in
    tmj)
        BINARY="tmj_terminal"
        PKG_NAME="tmj_${VERSION}_${DEB_ARCH}"
        DEPENDS="libasound2"
        DESCRIPTION="TerminalLove - A pixel-art visual novel engine rendered in the terminal"
        ;;
    tmj-wgpu)
        BINARY="tmj_wgpu"
        PKG_NAME="tmj-wgpu_${VERSION}_${DEB_ARCH}"
        DEPENDS="libasound2, libx11-6, libxcb1, libxkbcommon0"
        DESCRIPTION="TerminalLove - GPU windowed mode"
        ;;
    *) echo "Invalid variant: $VARIANT (expected tmj or tmj-wgpu)"; exit 1 ;;
esac

STAGING="staging/${TARGET}"
PKG_ROOT="deb_pkg"

rm -rf "$PKG_ROOT"
mkdir -p "${PKG_ROOT}/DEBIAN"
mkdir -p "${PKG_ROOT}/usr/bin"
mkdir -p "${PKG_ROOT}/usr/share/doc/${VARIANT}"
mkdir -p "${PKG_ROOT}/usr/share/licenses/${VARIANT}"

cp "${STAGING}/${BINARY}"   "${PKG_ROOT}/usr/bin/"
cp "${STAGING}/README.md"   "${PKG_ROOT}/usr/share/doc/${VARIANT}/"
cp "${STAGING}/LICENSE"     "${PKG_ROOT}/usr/share/licenses/${VARIANT}/"

INSTALLED_SIZE=$(du -sk "$PKG_ROOT" | cut -f1)

cat > "${PKG_ROOT}/DEBIAN/control" << EOF
Package: ${VARIANT}
Version: ${VERSION}
Architecture: ${DEB_ARCH}
Maintainer: rabitank
Installed-Size: ${INSTALLED_SIZE}
Section: games
Priority: optional
Depends: ${DEPENDS}
Description: ${DESCRIPTION}
Homepage: https://github.com/rabitank/TerminalLove
EOF

if [ "$VARIANT" = "tmj-wgpu" ]; then
    echo "Recommends: libwayland-client0" >> "${PKG_ROOT}/DEBIAN/control"
fi

mkdir -p target/artifacts
dpkg-deb --build "$PKG_ROOT" "target/artifacts/${PKG_NAME}.deb"
rm -rf "$PKG_ROOT"

echo "Built target/artifacts/${PKG_NAME}.deb"

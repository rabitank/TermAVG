#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:?Usage: $0 <target-triple>}"
VERSION="${2:?Usage: $0 <target-triple> <version>}"
VARIANT="${3:?Usage: $0 <target-triple> <version> <tmj|tmj-wgpu>}"
VERSION="${VERSION#v}"

case "$TARGET" in
    x86_64-unknown-linux-gnu)   ARCH_LINUX="x86_64" ;;
    *) echo "Unsupported target for arch: $TARGET"; exit 1 ;;
esac

case "$VARIANT" in
    tmj)       BINARY="tmj_terminal"; PKG_TEMPLATE="PKGBUILD_tmj.in" ;;
    tmj-wgpu)  BINARY="tmj_wgpu";    PKG_TEMPLATE="PKGBUILD_tmj-wgpu.in" ;;
    *) echo "Invalid variant: $VARIANT"; exit 1 ;;
esac

STAGING="staging/${TARGET}"
PKG_DIR="pkg/arch"
WORKDIR="$(pwd)"

cp "${STAGING}/${BINARY}" "${STAGING}/README.md" "${STAGING}/LICENSE" "doc/logo.png" "${PKG_DIR}/"
sed "s/__VERSION__/${VERSION}/g" "${PKG_DIR}/${PKG_TEMPLATE}" > "${PKG_DIR}/PKGBUILD"

PACKAGE_NAME=$(grep '^pkgname=' "${PKG_DIR}/PKGBUILD" | cut -d= -f2)
ARTIFACT="${PACKAGE_NAME}-${VERSION}-1-${ARCH_LINUX}.pkg.tar.zst"

docker run --rm \
    -v "${WORKDIR}:/workspace" \
    -w "/workspace/${PKG_DIR}" \
    archlinux:latest \
    bash -c "
        pacman -Sy --noconfirm base-devel &&
        useradd -m builder &&
        echo 'builder ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers &&
        chown -R builder:builder /workspace/${PKG_DIR} &&
        sudo -u builder bash -c 'CARCH=${ARCH_LINUX} makepkg -d --noconfirm'
    "

sudo mkdir -p target/artifacts
sudo cp "${PKG_DIR}/${ARTIFACT}" target/artifacts/
sudo rm -f "${PKG_DIR}/${BINARY}" "${PKG_DIR}/README.md" "${PKG_DIR}/LICENSE" "${PKG_DIR}/PKGBUILD"

echo "Built target/artifacts/${ARTIFACT}"

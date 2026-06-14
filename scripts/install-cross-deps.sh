#!/usr/bin/env bash
set -euo pipefail

# 先安装原生依赖（amd64 可直接用默认源）
sudo apt-get update
sudo apt-get install -y libasound2-dev libx11-dev libxcb1-dev libxkbcommon-dev libwayland-dev

# 安装交叉编译器
sudo apt-get install -y gcc-aarch64-linux-gnu

# 重写 apt 源：分离 amd64 和 arm64
# security.ubuntu.com 不提供 arm64，需要从 ports.ubuntu.com 获取
sudo tee /etc/apt/sources.list.d/ubuntu.sources > /dev/null << 'SOURCES'
Types: deb
URIs: http://archive.ubuntu.com/ubuntu/
Suites: noble noble-updates noble-backports
Components: main restricted universe multiverse
Architectures: amd64
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg

Types: deb
URIs: http://security.ubuntu.com/ubuntu/
Suites: noble-security
Components: main restricted universe multiverse
Architectures: amd64
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg

Types: deb
URIs: http://ports.ubuntu.com/
Suites: noble noble-updates noble-backports noble-security
Components: main restricted universe multiverse
Architectures: arm64
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg
SOURCES

sudo dpkg --add-architecture arm64
sudo apt-get update
sudo apt-get install -y \
    libasound2-dev:arm64 \
    libx11-dev:arm64 \
    libxcb1-dev:arm64 \
    libxkbcommon-dev:arm64 \
    libwayland-dev:arm64

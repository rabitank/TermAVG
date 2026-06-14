#!/usr/bin/env bash
set -euo pipefail

sudo apt-get update
sudo apt-get install -y libasound2-dev libx11-dev libxcb1-dev libxkbcommon-dev libwayland-dev

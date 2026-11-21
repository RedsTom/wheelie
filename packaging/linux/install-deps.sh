#!/usr/bin/env bash
# Installs the shared Linux build/packaging dependencies.
#
# Called by .github/workflows/ci.yml and .github/workflows/release.yml on
# Ubuntu runners, and usable locally on Debian/Ubuntu with sudo available.
set -euo pipefail

sudo apt-get update
sudo apt-get install -y \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  libxkbcommon-dev \
  libxkbcommon-x11-dev \
  libx11-dev \
  libx11-xcb-dev \
  libxcb1-dev \
  libxcursor-dev \
  libxrandr-dev \
  libxi-dev \
  libgl1-mesa-dev \
  libwayland-dev \
  libxdo-dev \
  libfontconfig-dev \
  libssl-dev \
  libvulkan1 \
  rpm

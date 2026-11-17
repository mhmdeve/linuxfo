#!/usr/bin/env bash
set -euo pipefail
if ! command -v dnf >/dev/null 2>&1; then echo 'This helper supports Fedora/RHEL-family systems with dnf.' >&2; exit 2; fi
sudo dnf install -y gcc gcc-c++ make pkgconf-pkg-config gtk4-devel libadwaita-devel curl
if ! command -v cargo >/dev/null 2>&1; then curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; source "$HOME/.cargo/env"; fi
"$(dirname "$0")/build-linux.sh"

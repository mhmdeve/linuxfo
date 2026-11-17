#!/usr/bin/env bash
set -euo pipefail

# Install build/runtime dependencies on Debian or Ubuntu, then build LinuxFo.
if ! command -v apt-get >/dev/null 2>&1; then echo 'This helper supports Debian/Ubuntu only.' >&2; exit 2; fi
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libgtk-4-dev libadwaita-1-dev curl ca-certificates
if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi
"$(dirname "$0")/build-linux.sh"

#!/usr/bin/env bash
set -euo pipefail

# Build the current host architecture. GTK4/libadwaita must be available at build time.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
cargo build --release -p linuxfo
mkdir -p dist
install -m 0755 target/release/linuxfo dist/linuxfo
install -m 0644 packaging/linuxfo.desktop dist/linuxfo.desktop
install -m 0644 packaging/io.linuxfo.SystemInfo.metainfo.xml dist/io.linuxfo.SystemInfo.metainfo.xml
printf 'LinuxFo built for %s\n' "$(rustc -vV | awk -F': ' '/host:/ {print $2}')"

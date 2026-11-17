# LinuxFo

LinuxFo is a cross-distribution Linux system information centre. It brings hardware, operating-system, resource, storage, sensor, camera, network, and device diagnostics into one desktop application.

## Goals

- One coherent GTK desktop experience across Debian/Ubuntu, Fedora, Arch, openSUSE, Alpine, and other Linux distributions.
- Native builds for x86_64 and ARM64, with ARMv7 considered where the toolchain and dependencies permit.
- No assumptions about a particular package manager, init system, desktop environment, or sensor chipset.
- Capability-based discovery: unavailable hardware and permission-limited interfaces are reported honestly rather than treated as fatal errors.
- Reproducible CI builds, portable archives, distribution packages, and versioned GitHub releases.

## Technology

LinuxFo is written in Rust with GTK4/libadwaita. Linux hardware data is collected through standard kernel interfaces (/proc, /sys, udev where available) and optional utilities. The UI and collectors are separated so missing utilities or inaccessible devices cannot crash the application.

## Development

See CONTRIBUTING.md and docs/architecture.md.

Run locally with `cargo run -p linuxfo` after installing Rust and GTK4/libadwaita development libraries.

## Hardware visibility

Linux exposes hardware information through kernel interfaces, firmware tables, drivers, and optional userspace tools. No application can promise every sensor or camera on every machine: some devices require vendor drivers, firmware support, elevated permissions, or APIs unavailable in a particular environment. LinuxFo surfaces source and availability for each reading.

# Contributing to LinuxFo

## Principles

1. Prefer stable kernel interfaces and documented userspace APIs.
2. Treat every hardware capability as optional.
3. Collect without root by default; explain permission limitations.
4. Keep collectors independent from the UI and parsers testable.
5. Never execute command strings through a shell. Use explicit arguments and bounded timeouts.
6. Include milestone notes in commit messages.

## Local development

Install Rust stable, GTK4 development files, and libadwaita development files using your distribution's package manager. Package names differ by distribution.

Run `cargo fmt --all -- --check`, `cargo test --workspace`, and `cargo run -p linuxfo`.

## Cross-distribution approach

The application must not install packages itself. Build native packages against supported distro runtimes; provide portable formats separately. GTK and platform libraries may remain runtime dependencies in native packages. Do not call a package universal without testing it on target distributions.

## Commits

Use focused imperative messages, e.g. `docs: define platform architecture`, `feat: add overview dashboard`, and `ci: build x86_64 and ARM64`.

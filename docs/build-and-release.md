# Build and release

## Native development build

Install Rust stable and GTK4/libadwaita development packages for your distribution. Then run:

```sh
cargo build --release -p linuxfo
./target/release/linuxfo
```

Helpers are provided for Debian/Ubuntu (`scripts/install-debian.sh`) and Fedora (`scripts/install-fedora.sh`). Other distributions should install the equivalent GTK4/libadwaita and Rust development packages using their package manager.

## CI

`.github/workflows/ci.yml` formats, tests the portable core and builds the GUI on GitHub-hosted x86_64 and ARM64 Linux runners. A passing runner demonstrates that architecture's build, not universal compatibility with every distribution or device.

## Releases

Push a semantic version tag such as `v0.1.0`. `.github/workflows/release.yml` builds x86_64 and ARM64 archives and SHA-256 checksums, then publishes them as a GitHub Release. The archive is dynamically linked against GTK4/libadwaita on the build image; users may need compatible runtime libraries. Truly self-contained AppImage/Flatpak and native deb/rpm packages are follow-on packaging work because their runtime and dependency policies differ.

## Supported targets

- x86_64 Linux: CI build
- aarch64 Linux: CI build
- ARMv7: not yet advertised; GTK4/libadwaita and Rust toolchain build validation is required

The project intentionally does not bundle every distribution's system libraries into one binary. Doing so is neither safe nor portable for kernel, udev, graphics and camera interfaces. Hardware support is capability-driven and depends on kernel drivers and permissions.

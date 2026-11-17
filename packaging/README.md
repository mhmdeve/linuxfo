# Packaging status

The current CI pipeline builds GTK4/libadwaita-linked LinuxFo binaries for x86_64 and AArch64 and publishes compressed archives with SHA-256 checksums on version tags. Those archives are not fully self-contained: compatible GTK4/libadwaita runtime libraries are required on the target system.

LinuxFo is deliberately not a static binary. GTK, graphics backends, udev/device enumeration and kernel interfaces make a single static package an unreliable cross-distro strategy. Native packages and sandbox packages should be built with their target runtime model.

Planned packaging formats:

- Debian/Ubuntu: .deb built against supported Debian/Ubuntu runtime baselines.
- Fedora/openSUSE: RPM built in a compatible distro container/image.
- Arch: PKGBUILD using system GTK4/libadwaita dependencies.
- Flatpak: runtime/SDK manifest to bundle a known GTK runtime and grant only required device access.
- AppImage: appimagetool bundle with GTK runtime plugins and platform integration tested across target distributions.

Do not label any package universal until it has been tested on its stated distribution and architecture matrix.

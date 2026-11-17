# LinuxFo architecture

## Product areas

- **Overview:** host identity, uptime, kernel, distribution, CPU, memory, root filesystem, live resource utilisation.
- **CPU:** model, architecture, topology, frequency and cache metadata when exposed.
- **Memory:** installed/available/used memory, swap, DIMM details where firmware exposes them.
- **Graphics:** GPU devices, driver, PCI identity, VRAM and displays where exposed.
- **Storage:** block devices, partitions, filesystems, mount points, capacity, usage, rotational/removable flags, SMART data when supported.
- **Thermals & sensors:** every readable hwmon temperature, fan RPM, voltage and power channel. Preserve chip, label, channel and source path.
- **Cameras:** enumerate V4L2 devices and formats without activating a stream.
- **Network:** interfaces, link state, speed, driver and wireless details.
- **Audio:** ALSA/PipeWire devices where available.
- **USB & PCI:** enumerated buses and IDs.
- **Battery & power:** charge, health, energy and AC state.
- **Software:** distro, package manager presence, desktop/session, boot mode and virtualization hints.
- **Diagnostics:** missing tools, unsupported interfaces and permission limitations.

## Collection layers

1. Rust core owns typed models and pure parsers.
2. Collectors read `/proc`, `/sys`, `/etc/os-release`, `/dev` and udev metadata without requiring root.
3. Optional integrations (`lsblk`, `lscpu`, `lspci`, `udevadm`, `sensors`, `v4l2-ctl`, `smartctl`) enrich results if present. Their absence is not fatal.
4. UI presents live data and source-level diagnostics.
5. Packaging targets native distro formats and portable formats separately.

## Portability constraints

Do not depend on systemd or a single package manager. GTK4/libadwaita is a desktop runtime dependency for native packages; Flatpak/AppImage workflows are separate strategies. ARM64 is first-class. ARMv7 will be advertised only after CI proves the GTK/toolchain combination.

## Sensor semantics

Each reading includes an identifier, label, raw value, unit and origin. hwmon temperature inputs are conventionally millidegrees Celsius; conversion is explicit. Never infer missing readings or merge distinct channels. Availability depends on kernel drivers and permissions.

## Camera semantics

Inventory `/dev/video*` and V4L2 metadata without starting a stream. Device enumeration does not guarantee streaming access. Respect privacy and device permissions.

## Release matrix

Initial CI targets Linux x86_64 and aarch64. CI artifacts include portable bundles. Tagged releases publish versioned bundles and checksums. OS-specific packages may need dedicated distro build images; do not cross-compile GUI stacks blindly.

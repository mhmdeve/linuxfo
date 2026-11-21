# Changelog

All notable changes to LinuxFo are documented in this file.

This project follows a simplified version of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and uses the version number defined in the Cargo workspace.

## [0.2.0] - 2026-11-19

### Added
- Searchable process management with process-name, PID, and status filters.
- Confirmed process termination and restart actions.
- Dedicated Drivers and Journal pages for hardware-driver information and system logs.
- Live journal streaming with pause/resume and severity and text filters.
- Live Performance dashboard with per-core CPU graphs and network traffic rates.
- Persistent, restricted root daemon for SMART/NVMe disk diagnostics.
- systemd service integration and daemon packaging for Debian, RPM, and Arch builds.

### Fixed
- Device detection no longer incorrectly identifies fingerprint readers as generic devices.
- Fixed page scrolling, excess blank space, and scroll-position resets during refresh.
- Moved CPU monitoring into Performance and integrated Drive Health at the top of Storage.
- Improved SMART diagnostics to use the persistent daemon rather than repeatedly requesting authentication.
- Fixed process-action callback handling and restart confirmation behavior.
- Fixed package metadata and version consistency across workspace crates and distribution packages.
- Fixed manual release version selection and release reruns.

### Notes
- Process actions run with the current user's permissions and do not automatically elevate privileges.
- Restarting systemd-managed services or processes requiring special launch environments may not work reliably.
- The hardware diagnostics daemon exposes only its fixed, read-only disk-health operation.

## [0.1.0] - 2026-11-17

### Added
- Cross-distribution GTK4/libadwaita desktop application for Linux system information and hardware monitoring.
- System overview with processor, memory, storage, sensor, camera, and device summaries.
- Dedicated pages for CPU, memory, storage, disk health, thermals, cameras, devices, network, and processes.
- Live system monitoring with a three-second refresh interval.
- Disk health discovery using SMART and NVMe information.
- NVMe endurance reporting, including percentage used and remaining.
- Disk health and current disk-activity percentage indicators where the required data is available.
- One-shot privileged SMART inventory helper to reduce repeated authentication prompts while keeping the main application unprivileged.
- Appearance selector for following the system theme or choosing light and dark themes.
- x86_64 and AArch64 CI validation and Debian package builds.
- GitHub release workflow for versioned archives, Debian packages, and SHA-256 checksums.
- MIT open-source license with copyright notice for Muhammad Hamza Mushtaq.

### Improved
- Refined the sidebar into a rounded, floating navigation card.
- Improved sidebar navigation spacing, selected-state styling, and application logo display.
- Updated Disk Health cards to show health and activity metrics more clearly.
- Extended SMART result caching to reduce repeated scans and authentication prompts.
- Documented screenshots, build instructions, hardware visibility, and camera permission guidance in the README.

### Fixed
- Corrected SMART helper JSON deserialization and effective-UID detection.
- Fixed disk-activity percentage calculation and connected sampled activity data to the system snapshot.
- Fixed Clippy and Rust compilation issues in the disk-activity sampler.

### Notes
- Disk activity is a busy-time utilisation indicator, not a disk-speed benchmark.
- For SATA/SAS devices, a passed SMART overall-health assessment is not a percentage estimate of remaining lifespan. Health percentages are only shown when the available device data supports the displayed estimate.
- GTK4 and libadwaita runtime libraries are required by native desktop builds.
- Hardware visibility depends on kernel drivers, firmware, device permissions, and the interfaces exposed by each distribution.

# Changelog

All notable changes to LinuxFo are documented in this file.

This project follows a simplified version of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and uses the version number defined in the Cargo workspace.

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
- Fixed manual release workflow version detection so branch-triggered runs use the Cargo package version instead of the branch name.
- Fixed the release publishing job by checking out the repository before invoking GitHub CLI.

### Notes
- Disk activity is a busy-time utilisation indicator, not a disk-speed benchmark.
- For SATA/SAS devices, a passed SMART overall-health assessment is not a percentage estimate of remaining lifespan. Health percentages are only shown when the available device data supports the displayed estimate.
- GTK4 and libadwaita runtime libraries are required by native desktop builds.
- Hardware visibility depends on kernel drivers, firmware, device permissions, and the interfaces exposed by each distribution.

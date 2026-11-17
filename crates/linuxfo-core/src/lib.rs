//! LinuxFo's best-effort Linux system discovery layer.
//!
//! Discovery uses standard Linux kernel interfaces and never requires root.
//! Missing drivers, permissions, or devices are reported as missing data.

use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use sysinfo::{Disks, Networks, System};

#[derive(Debug, Clone, Serialize)]
pub struct HostInfo {
    pub hostname: String,
    pub os: String,
    pub kernel: String,
    pub architecture: String,
    pub desktop: String,
    pub uptime: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuInfo {
    pub model: String,
    pub logical_cores: usize,
    pub usage_percent: f32,
    pub frequency_mhz: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub filesystem: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub removable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskHealthInfo {
    pub device: String,
    pub model: String,
    pub kind: String,
    pub status: String,
    pub detail: String,
    pub temperature_c: Option<f64>,
    pub power_on_hours: Option<u64>,
    pub percentage_used: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SensorInfo {
    pub id: String,
    pub chip: String,
    pub label: String,
    pub kind: String,
    pub value: f64,
    pub unit: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub name: String,
    pub path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkInfo {
    pub name: String,
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub host: HostInfo,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub disk_health: Vec<DiskHealthInfo>,
    pub sensors: Vec<SensorInfo>,
    pub cameras: Vec<DeviceInfo>,
    pub usb_devices: Vec<DeviceInfo>,
    pub pci_devices: Vec<DeviceInfo>,
    pub networks: Vec<NetworkInfo>,
    pub processes: Vec<ProcessInfo>,
    pub diagnostics: Vec<String>,
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap_or_default().trim().to_string()
}

fn env_value(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}

fn os_pretty_name() -> String {
    read("/etc/os-release")
        .lines()
        .find_map(|line| line.strip_prefix("PRETTY_NAME="))
        .map(|value| value.trim_matches('"').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| std::env::consts::OS.to_string())
}

fn uptime_string() -> String {
    let seconds: u64 = read("/proc/uptime")
        .split_whitespace()
        .next()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);

    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else {
        format!("{hours}h {minutes}m")
    }
}

fn cpu_model() -> String {
    read("/proc/cpuinfo")
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            if matches!(
                key.trim(),
                "model name" | "Hardware" | "Processor" | "cpu model"
            ) {
                Some(value.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "CPU information unavailable".into())
}

fn memory_info() -> MemoryInfo {
    let mut values: HashMap<String, u64> = HashMap::new();

    for line in read("/proc/meminfo").lines() {
        if let Some((key, value)) = line.split_once(':') {
            let kib = value
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse::<u64>()
                .unwrap_or(0);
            values.insert(key.to_string(), kib.saturating_mul(1024));
        }
    }

    let total = *values.get("MemTotal").unwrap_or(&0);
    let available = *values.get("MemAvailable").unwrap_or(&0);
    let swap_total = *values.get("SwapTotal").unwrap_or(&0);
    let swap_free = *values.get("SwapFree").unwrap_or(&0);

    MemoryInfo {
        total_bytes: total,
        used_bytes: total.saturating_sub(available),
        available_bytes: available,
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_total.saturating_sub(swap_free),
    }
}

fn collect_sensors() -> Vec<SensorInfo> {
    let mut sensors = Vec::new();
    let root = Path::new("/sys/class/hwmon");
    let Ok(chips) = fs::read_dir(root) else {
        return sensors;
    };

    for chip_entry in chips.flatten() {
        let directory = chip_entry.path();
        let chip_name = read(directory.join("name"));
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };

        let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        paths.sort();

        for path in paths {
            let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            let (prefix, kind, unit) = if filename.starts_with("temp")
                && filename.ends_with("_input")
            {
                ("temp", "Temperature", "°C")
            } else if filename.starts_with("fan") && filename.ends_with("_input") {
                ("fan", "Fan speed", "RPM")
            } else if filename.starts_with("in") && filename.ends_with("_input") {
                ("in", "Voltage", "mV")
            } else if filename.starts_with("power") && filename.ends_with("_input") {
                ("power", "Power", "µW")
            } else if filename.starts_with("curr") && filename.ends_with("_input") {
                ("curr", "Current", "µA")
            } else {
                continue;
            };

            let raw = read(&path);
            let Ok(number) = raw.parse::<f64>() else {
                continue;
            };

            let base = filename.strip_suffix("_input").unwrap_or(filename);
            let label = read(directory.join(format!("{base}_label")));
            let label = if label.is_empty() {
                format!("{kind} {base}")
            } else {
                label
            };
            let value = if prefix == "temp" {
                number / 1000.0
            } else {
                number
            };

            sensors.push(SensorInfo {
                id: format!("{}:{filename}", directory.display()),
                chip: chip_name.clone(),
                label,
                kind: kind.to_string(),
                value,
                unit: unit.to_string(),
                source: path.display().to_string(),
            });
        }
    }

    sensors
}

fn camera_permission_detail(path: &Path) -> String {
    let Ok(metadata) = fs::metadata(path) else {
        return "Device node exists but its permissions could not be read".into();
    };
    let status = read("/proc/self/status");
    let effective_uid = status.lines().find(|line| line.starts_with("Uid:"))
        .and_then(|line| line.split_whitespace().nth(2))
        .and_then(|value| value.parse::<u32>().ok()).unwrap_or(u32::MAX);
    let mut groups: Vec<u32> = status.lines()
        .find(|line| line.starts_with("Groups:"))
        .into_iter().flat_map(|line| line.split_whitespace().skip(1))
        .filter_map(|value| value.parse::<u32>().ok()).collect();
    if let Some(gid) = status.lines().find(|line| line.starts_with("Gid:"))
        .and_then(|line| line.split_whitespace().nth(2))
        .and_then(|value| value.parse::<u32>().ok()) {
        groups.push(gid);
    }
    let mode = metadata.permissions().mode();
    let readable = if effective_uid == metadata.uid() {
        mode & 0o400 != 0
    } else if groups.contains(&metadata.gid()) {
        mode & 0o040 != 0
    } else {
        mode & 0o004 != 0
    };
    if readable {
        format!("Read permission appears available • {}:{:o}; stream not opened", metadata.gid(), mode & 0o777)
    } else {
        format!("No read permission detected • {}:{:o}. Add your account to the device group (often 'video') and sign in again.", metadata.gid(), mode & 0o777)
    }
}

fn collect_video_devices() -> Vec<DeviceInfo> {
    let mut devices = Vec::new();
    let Ok(entries) = fs::read_dir("/dev") else {
        return devices;
    };

    for entry in entries.flatten() {
        let filename = entry.file_name().to_string_lossy().to_string();
        if !filename.starts_with("video") {
            continue;
        }

        let path = entry.path();
        let sysfs_name = PathBuf::from(format!(
            "/sys/class/video4linux/{filename}/name"
        ));
        let label = read(sysfs_name);
        let name = if label.is_empty() {
            filename
        } else {
            label
        };

        devices.push(DeviceInfo {
            name,
            path: path.display().to_string(),
            detail: camera_permission_detail(&path),
        });
    }

    devices.sort_by(|left, right| left.path.cmp(&right.path));
    devices
}

fn collect_bus(root: &str, bus: &str) -> Vec<DeviceInfo> {
    let mut devices = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return devices;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let id = entry.file_name().to_string_lossy().to_string();
        let vendor = read(path.join("idVendor"));
        let product = read(path.join("idProduct"));

        let detail = if bus == "USB" {
            format!("USB {vendor}:{product}")
        } else {
            format!("PCI {}", read(path.join("class")))
        };

        let product_name = read(path.join("product"));
        let name = if product_name.is_empty() {
            id
        } else {
            product_name
        };

        devices.push(DeviceInfo {
            name,
            path: path.display().to_string(),
            detail,
        });
    }

    devices.sort_by(|left, right| left.name.cmp(&right.name));
    devices
}


/// Read SMART/NVMe health using smartmontools when installed and permitted.
fn collect_disk_health_uncached() -> Vec<DiskHealthInfo> {
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/block") else { return result; };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("zram")
            || name.starts_with("dm-") { continue; }
        let device = format!("/dev/{name}");
        let base = entry.path();
        let model = read(base.join("device/model"));
        let model = if model.is_empty() { name.clone() } else { model };
        let kind = if name.starts_with("nvme") { "NVMe SSD" }
            else if name.starts_with("sd") { "SATA/SAS or USB drive" }
            else if name.starts_with("mmc") { "eMMC / SD storage" }
            else if name.starts_with("vd") || name.starts_with("xvd") { "Virtual disk" }
            else { "Block device" }.to_string();

        let output = std::process::Command::new("smartctl")
            .args(["-a", "-j", &device]).output();
        let mut status = "SMART unavailable".to_string();
        let mut detail = "Install smartmontools and grant device read access to inspect drive health.".to_string();
        let mut temperature_c = None;
        let mut power_on_hours = None;
        let mut percentage_used = None;

        if let Ok(output) = output {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                let smart = &json["smart_status"]["passed"];
                let nvme = &json["nvme_smart_health_information_log"];
                let ata = &json["ata_smart_data"];
                if smart.as_bool() == Some(true) {
                    status = "Healthy".into();
                    detail = "SMART overall-health self-assessment passed.".into();
                } else if smart.as_bool() == Some(false) {
                    status = "Warning".into();
                    detail = "SMART reports a failing health assessment. Back up this drive immediately.".into();
                } else if !nvme.is_null() {
                    status = "Available".into();
                    detail = "NVMe health log received; overall SMART pass flag not supplied.".into();
                } else if !ata.is_null() {
                    status = "Available".into();
                    detail = "SMART data received; overall health flag not supplied.".into();
                } else if let Some(message) = json["smartctl"]["messages"].as_array()
                    .and_then(|a| a.first()).and_then(|v| v["string"].as_str()) {
                    detail = message.to_string();
                }
                temperature_c = nvme["temperature"].as_f64()
                    .or_else(|| json["temperature"]["current"].as_f64());
                power_on_hours = nvme["power_on_hours"].as_u64()
                    .or_else(|| json["power_on_time"]["hours"].as_u64());
                percentage_used = nvme["percentage_used"].as_u64();
                if let Some(warning) = nvme["critical_warning"].as_u64() {
                    if warning > 0 {
                        status = "Critical".into();
                        detail = format!("NVMe critical warning bitmask: {warning:#04x}. Back up data and investigate.");
                    }
                }
                if let Some(percent) = percentage_used {
                    if percent >= 100 {
                        status = "Endurance limit reached".into();
                        detail = format!("NVMe percentage_used is {percent}%; the rated endurance has been consumed.");
                    }
                }
                if !output.status.success() && status == "SMART unavailable" {
                    detail = "smartctl could not read this device. Check permissions, controller support, or run with elevated privileges.".into();
                }
            } else {
                detail = "smartctl returned data that could not be parsed.".into();
            }
        } else {
            detail = "smartmontools (smartctl) is not installed.".into();
        }

        result.push(DiskHealthInfo { device, model, kind, status, detail, temperature_c, power_on_hours, percentage_used });
    }
    result.sort_by(|a,b| a.device.cmp(&b.device));
    result
}

// SMART commands are comparatively slow; cache their results for one minute instead
// of invoking smartctl on every three-second UI refresh.
fn collect_disk_health() -> Vec<DiskHealthInfo> {
    use std::sync::{Mutex, OnceLock};
    use std::time::Instant;
    static CACHE: OnceLock<Mutex<(Option<Instant>, Vec<DiskHealthInfo>)>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new((None, Vec::new())));
    let Ok(mut guard) = cache.lock() else { return collect_disk_health_uncached(); };
    let fresh = guard.0.map(|time| time.elapsed().as_secs() < 60).unwrap_or(false);
    if !fresh {
        guard.1 = collect_disk_health_uncached();
        guard.0 = Some(Instant::now());
    }
    guard.1.clone()
}

/// Collect a best-effort snapshot using unprivileged, standard Linux interfaces.
pub fn collect() -> Snapshot {
    let mut system = System::new_all();
    collect_with_system(&mut system)
}

/// Refresh an existing sysinfo instance so CPU deltas and process counters are meaningful.
pub fn collect_with_system(system: &mut System) -> Snapshot {
    system.refresh_all();

    let cpus = system.cpus();
    let cpu_usage = if cpus.is_empty() {
        0.0
    } else {
        cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32
    };

    let disks = Disks::new_with_refreshed_list()
        .iter()
        .map(|disk| DiskInfo {
            name: disk.name().to_string_lossy().to_string(),
            mount_point: disk.mount_point().display().to_string(),
            filesystem: disk.file_system().to_string_lossy().to_string(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
            removable: disk.is_removable(),
        })
        .collect();

    let networks = Networks::new_with_refreshed_list()
        .iter()
        .map(|(name, data)| NetworkInfo {
            name: name.clone(),
            received_bytes: data.total_received(),
            transmitted_bytes: data.total_transmitted(),
        })
        .collect();

    let mut processes: Vec<ProcessInfo> = system.processes().iter().map(|(pid, process)| ProcessInfo {
        pid: pid.as_u32(),
        name: process.name().to_string_lossy().into_owned(),
        cpu_percent: process.cpu_usage(),
        memory_bytes: process.memory(),
        status: format!("{:?}", process.status()),
    }).collect();
    processes.sort_by(|a, b| b.cpu_percent.total_cmp(&a.cpu_percent));
    processes.truncate(150);

    Snapshot {
        host: HostInfo {
            hostname: System::host_name().unwrap_or_else(|| "Unknown host".into()),
            os: os_pretty_name(),
            kernel: System::kernel_version().unwrap_or_else(|| "Unknown".into()),
            architecture: System::cpu_arch(),
            desktop: {
                let current = env_value("XDG_CURRENT_DESKTOP");
                if current.is_empty() {
                    env_value("DESKTOP_SESSION")
                } else {
                    current
                }
            },
            uptime: uptime_string(),
        },
        cpu: CpuInfo {
            model: cpu_model(),
            logical_cores: cpus.len(),
            usage_percent: cpu_usage,
            frequency_mhz: cpus.first().map(|cpu| cpu.frequency()).unwrap_or(0),
        },
        memory: memory_info(),
        disks,
        disk_health: collect_disk_health(),
        sensors: collect_sensors(),
        cameras: collect_video_devices(),
        usb_devices: collect_bus("/sys/bus/usb/devices", "USB"),
        pci_devices: collect_bus("/sys/bus/pci/devices", "PCI"),
        networks,
        processes,
        diagnostics: vec![
            "Sensor coverage depends on kernel drivers and exported hwmon channels.".into(),
            "Camera inventory does not open devices or test video streaming.".into(),
            "Disk SMART/NVMe health requires smartmontools and suitable device permissions.".into(),
        ],
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut index = 0;

    while value >= 1024.0 && index < UNITS.len() - 1 {
        value /= 1024.0;
        index += 1;
    }

    format!("{value:.1} {}", UNITS[index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_format_is_readable() {
        assert_eq!(format_bytes(1024), "1.0 KiB");
    }

    #[test]
    fn collection_does_not_panic() {
        let snapshot = collect();
        assert!(!snapshot.host.architecture.is_empty());
    }
}

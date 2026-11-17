//! LinuxFo's portable, non-privileged Linux hardware discovery core.
//! Each subsystem is best-effort: missing kernel interfaces never abort a scan.

use serde::Serialize;
use std::{fs, path::{Path, PathBuf}, time::Duration};
use sysinfo::{Disks, Networks, System};

#[derive(Debug, Clone, Serialize)]
pub struct HostInfo {
    pub hostname: String, pub os: String, pub kernel: String, pub architecture: String,
    pub desktop: String, pub uptime: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct CpuInfo { pub model: String, pub logical_cores: usize, pub usage_percent: f32, pub frequency_mhz: u64 }
#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo { pub total_bytes: u64, pub used_bytes: u64, pub available_bytes: u64, pub swap_total_bytes: u64, pub swap_used_bytes: u64 }
#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo { pub name: String, pub mount_point: String, pub filesystem: String, pub total_bytes: u64, pub available_bytes: u64, pub removable: bool }
#[derive(Debug, Clone, Serialize)]
pub struct SensorInfo { pub id: String, pub chip: String, pub label: String, pub kind: String, pub value: f64, pub unit: String, pub source: String }
#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo { pub name: String, pub path: String, pub detail: String }
#[derive(Debug, Clone, Serialize)]
pub struct NetworkInfo { pub name: String, pub received_bytes: u64, pub transmitted_bytes: u64 }
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub host: HostInfo, pub cpu: CpuInfo, pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>, pub sensors: Vec<SensorInfo>,
    pub cameras: Vec<DeviceInfo>, pub usb_devices: Vec<DeviceInfo>,
    pub pci_devices: Vec<DeviceInfo>, pub networks: Vec<NetworkInfo>,
    pub diagnostics: Vec<String>,
}

fn read(path: impl AsRef<Path>) -> String { fs::read_to_string(path).unwrap_or_default().trim().to_string() }
fn env(key: &str) -> String { std::env::var(key).unwrap_or_default() }
fn os_pretty_name() -> String {
    read("/etc/os-release").lines().find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"').to_string()).filter(|v| !v.is_empty())
        .unwrap_or_else(|| std::env::consts::OS.to_string())
}
fn uptime_string() -> String {
    let secs: u64 = read("/proc/uptime").split_whitespace().next().unwrap_or("0").parse().unwrap_or(0);
    let days=secs/86400; let hours=(secs%86400)/3600; let mins=(secs%3600)/60;
    if days>0 { format!("{days}d {hours}h {mins}m") } else { format!("{hours}h {mins}m") }
}
fn cpu_model() -> String {
    read("/proc/cpuinfo").lines().find_map(|l| {
        let (k,v)=l.split_once(':')?;
        if matches!(k.trim(), "model name" | "Hardware" | "Processor" | "cpu model") { Some(v.trim().to_string()) } else { None }
    }).unwrap_or_else(|| "CPU information unavailable".into())
}
fn memory_info() -> MemoryInfo {
    let mut vals=std::collections::HashMap::new();
    for l in read("/proc/meminfo").lines() { if let Some((k,v))=l.split_once(':') { let n=v.split_whitespace().next().unwrap_or("0").parse::<u64>().unwrap_or(0); vals.insert(k.to_string(),n*1024); } }
    let total=*vals.get("MemTotal").unwrap_or(&0); let avail=*vals.get("MemAvailable").unwrap_or(&0);
    let swap_total=*vals.get("SwapTotal").unwrap_or(&0); let swap_free=*vals.get("SwapFree").unwrap_or(&0);
    MemoryInfo { total_bytes:total, used_bytes:total.saturating_sub(avail), available_bytes:avail, swap_total_bytes:swap_total, swap_used_bytes:swap_total.saturating_sub(swap_free) }
}
fn collect_sensors() -> Vec<SensorInfo> {
    let mut out=Vec::new(); let root=Path::new("/sys/class/hwmon");
    let Ok(chips)=fs::read_dir(root) else { return out; };
    for chip in chips.flatten() {
        let dir=chip.path(); let chip_name=read(dir.join("name"));
        let Ok(entries)=fs::read_dir(&dir) else { continue; };
        let mut paths:Vec<PathBuf>=entries.flatten().map(|e|e.path()).collect(); paths.sort();
        for p in paths {
            let Some(file)=p.file_name().and_then(|x|x.to_str()) else { continue; };
            let (prefix,kind,unit)=if file.starts_with("temp") && file.ends_with("_input") { ("temp","Temperature","°C") }
                else if file.starts_with("fan") && file.ends_with("_input") { ("fan","Fan speed","RPM") }
                else if file.starts_with("in") && file.ends_with("_input") { ("in","Voltage","mV") }
                else if file.starts_with("power") && file.ends_with("_input") { ("power","Power","µW") }
                else if file.starts_with("curr") && file.ends_with("_input") { ("curr","Current","µA") } else { continue };
            let raw=read(&p); let Ok(n)=raw.parse::<f64>() else { continue; };
            let base=file.strip_suffix("_input").unwrap_or(file);
            let label=read(dir.join(format!("{base}_label")));
            let label=if label.is_empty(){format!("{kind} {base}")}else{label};
            let value=if prefix=="temp" { n/1000.0 } else { n };
            out.push(SensorInfo{id:format!("{}:{file}",dir.display()),chip:chip_name.clone(),label,kind:kind.into(),value,unit:unit.into(),source:p.display().to_string()});
        }
    }
    out
}
fn collect_video_devices() -> Vec<DeviceInfo> {
    let mut out=Vec::new(); let Ok(entries)=fs::read_dir("/dev") else{return out};
    for e in entries.flatten(){let name=e.file_name().to_string_lossy().to_string(); if name.starts_with("video") {
        let path=e.path(); let sys=PathBuf::from(format!("/sys/class/video4linux/{name}/name"));
        let label=read(sys); let label=if label.is_empty(){name.clone()}else{label};
        out.push(DeviceInfo{name:label,path:path.display().to_string(),detail:if path.exists(){"Device node present; streaming capability not tested".into()}else{"Unavailable".into()}});
    }} out.sort_by(|a,b|a.path.cmp(&b.path)); out
}
fn collect_bus(root:&str)->Vec<DeviceInfo>{
    let mut out=Vec::new(); let Ok(entries)=fs::read_dir(root) else{return out};
    for e in entries.flatten(){let p=e.path();let id=e.file_name().to_string_lossy().to_string();
        let vendor=read(p.join("idVendor")); let product=read(p.join("idProduct"));
        let detail=if !vendor.is_empty(){format!("USB {vendor}:{product}")}else{format!("PCI {}",read(p.join("class")))};
        let product_name=read(p.join("product")); let name=if product_name.is_empty(){id.clone()}else{product_name};
        out.push(DeviceInfo{name,path:p.display().to_string(),detail}); }
    out.sort_by(|a,b|a.name.cmp(&b.name)); out
}

/// Collect a best-effort snapshot using unprivileged, standard Linux interfaces.
pub fn collect() -> Snapshot {
    let mut system=System::new_all(); system.refresh_all(); std::thread::sleep(Duration::from_millis(120));
    system.refresh_cpu_usage(); system.refresh_memory();
    let cpus=system.cpus();
    let cpu_usage=if cpus.is_empty(){0.0}else{cpus.iter().map(|c|c.cpu_usage()).sum::<f32>()/cpus.len() as f32};
    let mut disks=Disks::new_with וח? ();
    let _=&mut disks;
    Snapshot {
        host:HostInfo{hostname:System::host_name().unwrap_or_else(||"Unknown host".into()),os:os_pretty_name(),kernel:System::kernel_version().unwrap_or_else(||"Unknown".into()),architecture:System::cpu_arch(),desktop:if env("XDG_CURRENT_DESKTOP").is_empty(){env("DESKTOP_SESSION")}else{env("XDG_CURRENT_DESKTOP")},uptime:uptime_string()},
        cpu:CpuInfo{model:cpu_model(),logical_cores:cpus.len(),usage_percent:cpu_usage,frequency_mhz:cpus.first().map(|c|c.frequency()).unwrap_or(0)},
        memory:{let m=memory_info(); MemoryInfo{total_bytes:m.total_bytes,used_bytes:m.used_bytes,available_bytes:m.available_bytes,swap_total_bytes:m.swap_total_bytes,swap_used_bytes:m.swap_used_bytes}},
        disks:{let ds=Disks::new_with_refreshed_list();ds.iter().map(|d|DiskInfo{name:d.name().to_string_lossy().to_string(),mount_point:d.mount_point().display().to_string(),filesystem:d.file_system().to_string_lossy().to_string(),total_bytes:d.total_space(),available_bytes:d.available_space(),removable:d.is_removable()}).collect()},
        sensors:collect_sensors(),cameras:collect_video_devices(),usb_devices:collect_bus("/sys/bus/usb/devices"),pci_devices:collect_bus("/sys/bus/pci/devices"),
        networks:{let ns=Networks::new_with_refreshed_list();ns.iter().map(|(n,d)|NetworkInfo{name:n.clone(),received_bytes:d.total_received(),transmitted_bytes:d.total_transmitted()}).collect()},
        diagnostics:vec![
            "Sensor coverage depends on kernel drivers and exported hwmon channels.".into(),
            "Camera inventory does not open devices or test video streaming.".into(),
            "SMART, detailed GPU telemetry and firmware DIMM data are planned optional integrations.".into(),
        ]
    }
}

pub fn format_bytes(n:u64)->String { const U:[&str;5]=["B","KiB","MiB","GiB","TiB"]; let mut v=n as f64;let mut i=0;while v>=1024.0&&i<U.len()-1{v/=1024.0;i+=1;}format!("{v:.1} {}",U[i]) }

#[cfg(test)] mod tests { use super::*; #[test] fn byte_format_is_readable(){assert_eq!(format_bytes(1024),"1.0 KiB");} #[test] fn collect_never_panics(){let s=collect();assert!(!s.host.architecture.is_empty());} }

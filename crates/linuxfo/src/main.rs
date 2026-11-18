use adw::prelude::*;
use gtk::{glib, Align, Orientation};
use linuxfo_core::{collect_with_system, format_bytes, Snapshot};
use sysinfo::System;
use std::{cell::RefCell, rc::Rc, time::Duration};

const CSS: &str = r#"
* { font-family: "Inter", "Noto Sans", "Cantarell", sans-serif; }
window { background: #e9ebe8; color: #252a26; }
headerbar { background: #e5e8e3; color: #252a26; border-bottom: 1px solid alpha(currentColor, 0.10); min-height: 54px; }
.sidebar { background: #f1f3ef; color: #252a26; border: 1px solid #d8ded7; border-radius: 22px; padding: 14px; box-shadow: 0 5px 18px alpha(#1c281e, 0.08); overflow: hidden; }
.brand { font-size: 23px; font-weight: 800; letter-spacing: -0.9px; color: #202621; }
.brand-mark { background: #343d36; color: #f4f5f1; border-radius: 11px; padding: 8px 9px; font-size: 13px; font-weight: 800; }
.nav-button { background: transparent; color: #69736b; border: 1px solid transparent; border-radius: 12px; padding: 0; min-height: 44px; font-size: 13px; font-weight: 600; text-align: left; box-shadow: none; }
.nav-button > box { padding: 0 12px; }
.nav-icon { min-width: 22px; font-size: 16px; color: #87938a; }
.nav-label { font-size: 13px; font-weight: 600; }
.brand-panel { background: alpha(currentColor, 0.035); border: 1px solid alpha(currentColor, 0.07); border-radius: 15px; padding: 12px; }
.brand-logo { min-width: 38px; min-height: 38px; }
.nav-section { margin-top: 8px; margin-bottom: 4px; }
.nav-button:hover { background: alpha(currentColor, 0.07); color: #252d27; }
.nav-button:checked { background: #dce8dd; color: #28633a; border: 1px solid #cbdccc; font-weight: 700; box-shadow: 0 1px 3px alpha(#203526, 0.07); }
.nav-button:checked .nav-icon { color: #34794a; }
.nav-button:checked .nav-label { color: #28633a; font-weight: 750; }
.page-title { font-size: 30px; font-weight: 750; letter-spacing: -1px; color: #202621; }
.muted { color: #727b74; font-size: 12px; }
.card { background: #f5f6f2; color: #252a26; border: 1px solid #d9ded8; border-radius: 15px; padding: 19px; box-shadow: 0 2px 7px alpha(#202820, 0.045); }
.card-title { color: #747d75; font-size: 11px; font-weight: 750; letter-spacing: 0.8px; }
.metric { font-size: 29px; font-weight: 750; letter-spacing: -0.8px; color: #202621; }
.metric-small { font-size: 14px; font-weight: 650; color: #2b332d; }
.accent { color: #34794a; font-weight: 700; }
.pill { background: #dce9dd; color: #28633a; border-radius: 20px; padding: 5px 9px; font-size: 11px; font-weight: 700; }
boxedlist, list { background: #f5f6f2; color: #252a26; border: 1px solid #d9ded8; border-radius: 13px; }
list row { background: transparent; border-bottom: 1px solid #e2e6e0; }
list row:last-child { border-bottom: none; }
progressbar trough { background: #dce1da; border-radius: 8px; min-height: 8px; }
progressbar progress { background: #438354; border-radius: 8px; min-height: 8px; }
separator { background: #d8ded7; }
scrolledwindow { background: transparent; }
window.dark { background: #202420; color: #e5e9e4; }
window.dark headerbar { background: #252b26; color: #e5e9e4; }
window.dark .sidebar { background: #252b26; color: #e5e9e4; border: 1px solid #3b463d; box-shadow: 0 5px 20px alpha(#000000, 0.18); }

window.dark .brand, window.dark .page-title, window.dark .metric { color: #edf1eb; }
window.dark .brand-mark { background: #dce8dc; color: #263229; }
window.dark .nav-button { color: #b7c0b8; }
window.dark .brand-panel { background: #202720; border-color: #39443b; }
window.dark .nav-icon { color: #849188; }
window.dark .nav-button:hover { background: #303a32; color: #ffffff; }
window.dark .nav-button:checked { background: #314637; color: #bce3c3; border-color: #405947; }
window.dark .nav-button:checked .nav-label, window.dark .nav-button:checked .nav-icon { color: #bce3c3; }
window.dark .card, window.dark boxedlist, window.dark list { background: #2b322c; color: #e5e9e4; border-color: #414a42; }
window.dark .metric-small { color: #e0e7df; }
window.dark .muted { color: #a0aaa1; }
window.dark .card-title { color: #aab5ab; }
window.dark .accent { color: #91d3a0; }
window.dark progressbar trough { background: #465047; }
window.dark progressbar progress { background: #75b985; }
window.dark separator { background: #414a42; }
"#;

fn label(text: &str, class: &str) -> gtk::Label {
    let widget = gtk::Label::new(Some(text));
    widget.set_xalign(0.0);
    widget.add_css_class(class);
    widget
}

fn metric_card(title: &str, value: &str, subtitle: &str) -> gtk::Box {
    let widget = gtk::Box::new(Orientation::Vertical, 8);
    widget.add_css_class("card");
    widget.append(&label(&title.to_uppercase(), "card-title"));
    widget.append(&label(value, "metric"));
    widget.append(&label(subtitle, "muted"));
    widget
}

fn section_header(title: &str, subtitle: &str) -> gtk::Box {
    let widget = gtk::Box::new(Orientation::Vertical, 4);
    widget.append(&label(title, "page-title"));
    widget.append(&label(subtitle, "muted"));
    widget
}

fn info_row(title: &str, value: &str, detail: &str) -> gtk::Box {
    let widget = gtk::Box::new(Orientation::Horizontal, 12);
    widget.add_css_class("card");
    widget.set_hexpand(true);

    let left = gtk::Box::new(Orientation::Vertical, 4);
    left.set_hexpand(true);
    left.append(&label(title, "metric-small"));
    left.append(&label(detail, "muted"));
    widget.append(&left);

    let right = label(value, "metric-small");
    right.add_css_class("accent");
    right.set_valign(Align::Center);
    widget.append(&right);
    widget
}


fn sysfs_value(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).map(|v| v.trim().to_string()).unwrap_or_else(|_| "—".into())
}

fn page_scroller(child: &gtk::Widget) -> gtk::ScrolledWindow {
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hexpand(true);
    scroller.set_vexpand(true);
    scroller.set_child(Some(child));
    scroller
}

fn reset_visible_page_scroll(stack: &gtk::Stack) {
    if let Some(scroller) = stack.visible_child()
        .and_then(|child| child.downcast::<gtk::ScrolledWindow>().ok())
    {
        scroller.vadjustment().set_value(0.0);
    }
}

fn driver_inventory_page() -> gtk::Widget {
    let root = gtk::Box::new(Orientation::Vertical, 14);
    root.append(&section_header("Drivers & Hardware", "Kernel drivers and device bindings reported by Linux sysfs"));

    let mut entries: Vec<(String, String, String)> = Vec::new();
    for (bus, title) in [("pci", "PCI"), ("usb", "USB")] {
        let base = std::path::PathBuf::from(format!("/sys/bus/{bus}/devices"));
        if let Ok(devices) = std::fs::read_dir(base) {
            for device in devices.flatten() {
                let path = device.path();
                let id = device.file_name().to_string_lossy().to_string();
                let vendor = sysfs_value(&path.join("vendor"));
                let product = sysfs_value(&path.join("device"));
                let class = sysfs_value(&path.join("class"));
                let driver = std::fs::read_link(path.join("driver"))
                    .ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .unwrap_or_else(|| "No driver bound".into());
                let modalias = sysfs_value(&path.join("modalias"));
                entries.push((format!("{title} • {id}"), format!("Vendor {vendor} • Device {product} • Class {class}"), format!("Driver: {driver} • {modalias}")));
            }
        }
    }

    if let Ok(interfaces) = std::fs::read_dir("/sys/class/net") {
        for interface in interfaces.flatten() {
            let path = interface.path();
            let name = interface.file_name().to_string_lossy().to_string();
            let driver = std::fs::read_link(path.join("device/driver"))
                .ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "Virtual / driver not exposed".into());
            let address = sysfs_value(&path.join("address"));
            let wireless = path.join("wireless").exists() || path.join("phy80211").exists();
            entries.push((format!("Network • {name}"), if wireless { "Wi-Fi interface".into() } else { "Network interface".into() }, format!("Driver: {driver} • MAC: {address}")));
        }
    }

    if let Ok(radios) = std::fs::read_dir("/sys/class/bluetooth") {
        for radio in radios.flatten() {
            let name = radio.file_name().to_string_lossy().to_string();
            let path = radio.path();
            let driver = std::fs::read_link(path.join("device/driver"))
                .ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "Driver not exposed".into());
            entries.push((format!("Bluetooth • {name}"), "Bluetooth controller".into(), format!("Driver: {driver}")));
        }
    }

    // Fingerprint reader support is not inferred from USB presence alone:
    // device compatibility must be checked against libfprint's supported-device database.
    entries.sort_by(|a,b| a.0.cmp(&b.0));
    if entries.is_empty() {
        root.append(&info_row("No driver-bound devices discovered", "—", "LinuxFo could not read PCI, USB, network or Bluetooth sysfs entries."));
    } else {
        root.append(&label(&format!("{} device and interface entries", entries.len()), "muted"));
        for (name, detail, driver) in entries {
            root.append(&info_row(&name, &detail, &driver));
        }
    }
    root.append(&label("Fingerprint support depends on the device being supported by libfprint/fprintd. A USB device may be listed above without being a supported fingerprint reader.", "muted"));
    root.upcast()
}

fn journal_page() -> gtk::Widget {
    let root = gtk::Box::new(Orientation::Vertical, 12);
    root.append(&section_header("System Journal", "Recent systemd journal entries with severity and text filters"));

    let controls = gtk::Box::new(Orientation::Horizontal, 8);
    let severity = gtk::DropDown::from_strings(&["All", "Critical", "Error", "Warning", "Notice", "Info", "Debug"]);
    severity.set_hexpand(false);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Filter log text…"));
    search.set_hexpand(true);
    let refresh = gtk::Button::with_label("Load journal");
    controls.append(&severity);
    controls.append(&search);
    controls.append(&refresh);
    root.append(&controls);

    let output = gtk::TextView::new();
    output.set_editable(false);
    output.set_cursor_visible(false);
    output.set_monospace(true);
    output.set_wrap_mode(gtk::WrapMode::None);
    output.set_vexpand(true);
    let buffer = output.buffer();
    buffer.set_text("Select a severity and press “Load journal”. The journal is read with your current user permissions.");
    let log_scroll = gtk::ScrolledWindow::new();
    log_scroll.set_min_content_height(420);
    log_scroll.set_vexpand(true);
    log_scroll.set_hexpand(true);
    log_scroll.set_child(Some(&output));
    root.append(&log_scroll);

    let severity_ref = severity.clone();
    let search_ref = search.clone();
    let buffer_ref = buffer.clone();
    refresh.connect_clicked(move |_| {
        let priority = match severity_ref.selected() {
            1 => Some("crit"),
            2 => Some("err"),
            3 => Some("warning"),
            4 => Some("notice"),
            5 => Some("info"),
            6 => Some("debug"),
            _ => None,
        };
        let mut command = std::process::Command::new("journalctl");
        command.args(["--no-pager", "-n", "500", "-o", "short-iso"]);
        if let Some(priority) = priority {
            command.args(["-p", priority]);
        }
        let text = match command.output() {
            Ok(result) if result.status.success() => {
                let all = String::from_utf8_lossy(&result.stdout);
                let needle = search_ref.text().to_string().to_lowercase();
                let filtered: Vec<&str> = all.lines().filter(|line| needle.is_empty() || line.to_lowercase().contains(&needle)).collect();
                if filtered.is_empty() { "No journal entries matched the selected filters.".into() }
                else { filtered.join("\n") }
            }
            Ok(result) => format!("journalctl failed:\n{}", String::from_utf8_lossy(&result.stderr)),
            Err(error) => format!("Could not execute journalctl: {error}\nInstall systemd tools or check your PATH."),
        };
        buffer_ref.set_text(&text);
    });
    root.append(&label("Access note: add your user to the systemd-journal group for read access to the system journal, then sign out and back in. LinuxFo does not run journalctl as root.", "muted"));
    root.upcast()
}

fn content_page(snapshot: &Snapshot, page: &str, monitor: Rc<RefCell<System>>, process_filter: Rc<RefCell<String>>) -> gtk::Widget {
    let root = gtk::Box::new(Orientation::Vertical, 18);
    root.set_margin_top(28);
    root.set_margin_bottom(28);
    root.set_margin_start(30);
    root.set_margin_end(30);

    let (title, subtitle) = match page {
        "CPU" => ("Processor", "Architecture, topology and utilisation"),
        "Memory" => ("Memory", "Physical memory and swap usage"),
        "Storage" => ("Storage", "Filesystems, devices and mount points"),
        "Disk Health" => ("Disk Health", "SMART and NVMe drive condition, temperature and endurance"),
        "Thermals" => ("Thermals & Sensors", "Kernel-exposed temperature and telemetry channels"),
        "Cameras" => ("Cameras", "Video devices discovered without activating streams"),
        "Devices" => ("Hardware Devices", "USB and PCI device inventory"),
        "Network" => ("Network", "Interface traffic counters"),
        "Processes" => ("Processes", "Live process activity, CPU load and resident memory"),
        "Drivers" => ("Drivers & Hardware", "Kernel driver bindings and device interfaces"),
        "Journal" => ("System Journal", "Recent logs filtered by severity and text"),
        _ => ("System Overview", "A live view of your Linux machine"),
    };

    root.append(&section_header(title, subtitle));

    match page {
        "CPU" => {
            root.append(&info_row(
                "Processor",
                &snapshot.cpu.model,
                &format!(
                    "{} logical CPUs • {} MHz",
                    snapshot.cpu.logical_cores, snapshot.cpu.frequency_mhz
                ),
            ));
            root.append(&info_row(
                "Current utilisation",
                &format!("{:.1}%", snapshot.cpu.usage_percent),
                "Average across logical CPUs",
            ));
        }
        "Memory" => {
            root.append(&info_row(
                "Physical memory",
                &format!(
                    "{} / {}",
                    format_bytes(snapshot.memory.used_bytes),
                    format_bytes(snapshot.memory.total_bytes)
                ),
                &format!(
                    "{} available",
                    format_bytes(snapshot.memory.available_bytes)
                ),
            ));
            root.append(&info_row(
                "Swap",
                &format!(
                    "{} / {}",
                    format_bytes(snapshot.memory.swap_used_bytes),
                    format_bytes(snapshot.memory.swap_total_bytes)
                ),
                "Kernel swap accounting",
            ));
        }
        "Storage" => {
            for disk in &snapshot.disks {
                let used = disk.total_bytes.saturating_sub(disk.available_bytes);
                root.append(&info_row(
                    &disk.name,
                    &format_bytes(disk.total_bytes),
                    &format!(
                        "{} • {} • {} used • {} free{}",
                        disk.mount_point,
                        disk.filesystem,
                        format_bytes(used),
                        format_bytes(disk.available_bytes),
                        if disk.removable { " • removable" } else { "" }
                    ),
                ));
            }
        }
        "Disk Health" => {
            if snapshot.disk_health.is_empty() {
                root.append(&info_row("No physical drives found", "—", "No supported block devices were discovered under /sys/block."));
            } else {
                for disk in &snapshot.disk_health {
                    let mut metrics = Vec::new();
                    if let Some(temp) = disk.temperature_c { metrics.push(format!("{temp:.0} °C")); }
                    if let Some(hours) = disk.power_on_hours { metrics.push(format!("{hours} power-on hours")); }
                    if let Some(used) = disk.percentage_used { metrics.push(format!("{used}% endurance used")); }
                    let extra = if metrics.is_empty() { disk.detail.clone() } else { format!("{} • {}", metrics.join(" • "), disk.detail) };
                    let card = gtk::Box::new(Orientation::Vertical, 10);
                    card.add_css_class("card");
                    let heading = gtk::Box::new(Orientation::Horizontal, 8);
                    let identity = gtk::Box::new(Orientation::Vertical, 3);
                    identity.set_hexpand(true);
                    identity.append(&label(&format!("{} • {}", disk.device, disk.model), "metric-small"));
                    identity.append(&label(&disk.kind, "muted"));
                    heading.append(&identity);
                    heading.append(&label(&disk.status, "accent"));
                    card.append(&heading);
                    let metrics_grid = gtk::Grid::new();
                    metrics_grid.set_column_spacing(18);
                    metrics_grid.set_row_spacing(8);
                    let health_text = disk.health_percent.map(|v| format!("{:.0}%", v.clamp(0.0, 100.0))).unwrap_or_else(|| "Unavailable".into());
                    let performance_text = disk.performance_percent.map(|v| format!("{:.0}%", v.clamp(0.0, 100.0))).unwrap_or_else(|| "Sampling…".into());
                    metrics_grid.attach(&label("HEALTH", "card-title"), 0, 0, 1, 1);
                    metrics_grid.attach(&label("DISK ACTIVITY", "card-title"), 1, 0, 1, 1);
                    metrics_grid.attach(&label(&health_text, "metric-small"), 0, 1, 1, 1);
                    metrics_grid.attach(&label(&performance_text, "metric-small"), 1, 1, 1, 1);
                    for (column, value) in [(0, disk.health_percent), (1, disk.performance_percent)] {
                        if let Some(value) = value {
                            let bar = gtk::ProgressBar::new();
                            bar.set_fraction(value.clamp(0.0, 100.0) / 100.0);
                            metrics_grid.attach(&bar, column, 2, 1, 1);
                        }
                    }
                    card.append(&metrics_grid);
                    let health_note = if disk.percentage_used.is_some() {
                        "NVMe health is derived from remaining rated endurance"
                    } else if disk.health_percent.is_some() {
                        "Health reflects the drive's SMART pass/fail assessment, not remaining lifespan"
                    } else {
                        "Drive does not expose a usable SMART health percentage"
                    };
                    card.append(&label(&format!("{} • Disk activity is busy time, not a speed score • {}", extra, health_note), "muted"));
                    root.append(&card);
                }
            }
        }
        "Thermals" => {
            if snapshot.sensors.is_empty() {
                root.append(&info_row(
                    "No hwmon channels found",
                    "—",
                    "The current kernel or hardware does not expose readable sensor inputs.",
                ));
            } else {
                for sensor in &snapshot.sensors {
                    root.append(&info_row(
                        &sensor.label,
                        &format!("{:.1} {}", sensor.value, sensor.unit),
                        &format!("{} • {} • {}", sensor.kind, sensor.chip, sensor.source),
                    ));
                }
            }
        }
        "Cameras" => {
            if snapshot.cameras.is_empty() {
                root.append(&info_row(
                    "No video devices found",
                    "—",
                    "No /dev/video* nodes were discovered. Check driver and device permissions.",
                ));
            } else {
                for device in &snapshot.cameras {
                    root.append(&info_row(&device.name, &device.path, &device.detail));
                }
            }
        }
        "Devices" => {
            for device in snapshot.usb_devices.iter().chain(snapshot.pci_devices.iter()) {
                root.append(&info_row(&device.name, &device.detail, &device.path));
            }
        }
        "Processes" => {
            let search = gtk::SearchEntry::new();
            search.set_hexpand(true);
            search.set_placeholder_text(Some("Search by process name, PID or status…"));
            search.set_text(&process_filter.borrow());
            let filter_state = process_filter.clone();
            search.connect_search_changed(move |entry| {
                *filter_state.borrow_mut() = entry.text().to_string();
            });
            root.append(&search);

            let needle = process_filter.borrow().to_lowercase();
            let matching: Vec<_> = snapshot.processes.iter().filter(|process| {
                needle.is_empty()
                    || process.name.to_lowercase().contains(&needle)
                    || process.pid.to_string().contains(&needle)
                    || process.status.to_lowercase().contains(&needle)
            }).collect();
            root.append(&label(
                &format!("{} matching processes • sorted by CPU usage • refreshes every 3 seconds", matching.len()),
                "muted",
            ));
            let list = gtk::ListBox::new();
            list.add_css_class("boxed-list");
            for process in matching {
                let row = gtk::Box::new(Orientation::Horizontal, 10);
                row.set_margin_top(8);
                row.set_margin_bottom(8);
                row.set_margin_start(12);
                row.set_margin_end(12);

                let identity = gtk::Box::new(Orientation::Vertical, 3);
                identity.set_hexpand(true);
                identity.append(&label(&process.name, "metric-small"));
                identity.append(&label(&format!("PID {} • {}", process.pid, process.status), "muted"));
                row.append(&identity);
                row.append(&label(&format!("{:.1}% CPU", process.cpu_percent), "accent"));
                row.append(&label(&format_bytes(process.memory_bytes), "muted"));

                let kill_button = gtk::Button::with_label("Kill");
                kill_button.add_css_class("destructive-action");
                let action_monitor = monitor.clone();
                let target_pid = process.pid;
                let target_name = process.name.clone();
                kill_button.connect_clicked(move |button| {
                    let Some(window) = button.root().and_downcast::<gtk::Window>() else { return; };
                    let dialog = gtk::MessageDialog::builder()
                        .transient_for(&window)
                        .modal(true)
                        .message_type(gtk::MessageType::Warning)
                        .buttons(gtk::ButtonsType::YesNo)
                        .text(format!("Kill {} (PID {})?", target_name, target_pid))
                        .secondary_text("This sends SIGKILL. Unsaved work in this process may be lost.")
                        .build();
                    let action_monitor = action_monitor.clone();
                    dialog.connect_response(move |dialog, response| {
                        if response == gtk::ResponseType::Yes {
                            if let Some((_, target)) = action_monitor.borrow().processes().iter()
                                .find(|(pid, _)| pid.as_u32() == target_pid) {
                                target.kill();
                            }
                        }
                        dialog.close();
                    });
                    dialog.present();
                });
                row.append(&kill_button);

                let restart_button = gtk::Button::with_label("Restart");
                let action_monitor = monitor.clone();
                let restart_pid = process.pid;
                restart_button.set_sensitive(restart_pid > 1);
                restart_button.connect_clicked(move |button| {
                    let Some(window) = button.root().and_downcast::<gtk::Window>() else { return; };
                    let restart_data = {
                        let system = action_monitor.borrow();
                        system.processes().iter()
                            .find(|(pid, _)| pid.as_u32() == restart_pid)
                            .and_then(|(_, process)| {
                                Some((
                                    process.exe()?.to_path_buf(),
                                    process.cmd().iter().skip(1).cloned().collect::<Vec<_>>(),
                                    process.cwd().map(|path| path.to_path_buf()),
                                ))
                            })
                    };
                    let Some((executable, args, cwd)) = restart_data else {
                        let dialog = gtk::MessageDialog::builder()
                            .transient_for(&window)
                            .modal(true)
                            .message_type(gtk::MessageType::Warning)
                            .buttons(gtk::ButtonsType::Ok)
                            .text("This process cannot be restarted")
                            .secondary_text("LinuxFo could not read its executable path. Some kernel, protected or short-lived processes cannot be relaunched.")
                            .build();
                        dialog.connect_response(|dialog, _| dialog.close());
                        dialog.present();
                        return;
                    };
                    let dialog = gtk::MessageDialog::builder()
                        .transient_for(&window)
                        .modal(true)
                        .message_type(gtk::MessageType::Warning)
                        .buttons(gtk::ButtonsType::YesNo)
                        .text(format!("Restart PID {}?", restart_pid))
                        .secondary_text("LinuxFo will terminate this process and launch its executable again with the recorded arguments. This may not work correctly for systemd-managed services.")
                        .build();
                    let action_monitor = action_monitor.clone();
                    dialog.connect_response(move |dialog, response| {
                        if response == gtk::ResponseType::Yes {
                            let killed = action_monitor.borrow().processes().iter()
                                .find(|(pid, _)| pid.as_u32() == restart_pid)
                                .map(|(_, process)| process.kill())
                                .unwrap_or(false);
                            if killed {
                                // The GTK response callback is Fn and may be invoked more than once,
                                // so clone the launch data before moving it into the one-shot timer.
                                let executable = executable.clone();
                                let args = args.clone();
                                let cwd = cwd.clone();
                                glib::timeout_add_local_once(Duration::from_millis(700), move || {
                                    let mut command = std::process::Command::new(executable);
                                    command.args(args);
                                    if let Some(directory) = cwd {
                                        command.current_dir(directory);
                                    }
                                    let _ = command.spawn();
                                });
                            }
                        }
                        dialog.close();
                    });
                    dialog.present();
                });
                row.append(&restart_button);

                let item = gtk::ListBoxRow::new();
                item.set_child(Some(&row));
                list.append(&item);
            }
            root.append(&list);
            root.append(&label("Process actions run with your current user permissions. Protected processes may require administrator tools; LinuxFo does not elevate privileges.", "muted"));
        }
        "Drivers" => return driver_inventory_page(),
        "Journal" => return journal_page(),
        "Network" => {
            for network in &snapshot.networks {
                root.append(&info_row(
                    &network.name,
                    &format!(
                        "↓ {}  ↑ {}",
                        format_bytes(network.received_bytes),
                        format_bytes(network.transmitted_bytes)
                    ),
                    "Cumulative interface byte counters",
                ));
            }
        }
        _ => {
            let grid = gtk::Grid::new();
            grid.set_column_spacing(14);
            grid.set_row_spacing(14);
            grid.set_column_homogeneous(true);

            let memory_percent = if snapshot.memory.total_bytes == 0 {
                0.0
            } else {
                snapshot.memory.used_bytes as f64 * 100.0
                    / snapshot.memory.total_bytes as f64
            };

            let cards = [
                metric_card(
                    "Processor",
                    &format!("{:.0}%", snapshot.cpu.usage_percent),
                    &snapshot.cpu.model,
                ),
                metric_card(
                    "Memory",
                    &format!("{memory_percent:.1}%"),
                    &format!(
                        "{} of {} used",
                        format_bytes(snapshot.memory.used_bytes),
                        format_bytes(snapshot.memory.total_bytes)
                    ),
                ),
                metric_card(
                    "Storage",
                    &snapshot.disks.len().to_string(),
                    "Mounted filesystems discovered",
                ),
                metric_card(
                    "Sensors",
                    &snapshot.sensors.len().to_string(),
                    "Readable hwmon channels",
                ),
                metric_card(
                    "Cameras",
                    &snapshot.cameras.len().to_string(),
                    "Video devices discovered",
                ),
                metric_card(
                    "Devices",
                    &(snapshot.usb_devices.len() + snapshot.pci_devices.len()).to_string(),
                    "USB and PCI entries",
                ),
            ];

            for (index, card) in cards.iter().enumerate() {
                grid.attach(card, (index % 3) as i32, (index / 3) as i32, 1, 1);
            }
            root.append(&grid);

            root.append(&info_row(
                "Operating system",
                &snapshot.host.os,
                &format!(
                    "{} • {}",
                    snapshot.host.architecture, snapshot.host.kernel
                ),
            ));
            root.append(&info_row(
                "Machine",
                &snapshot.host.hostname,
                &format!(
                    "{} • uptime {}",
                    if snapshot.host.desktop.is_empty() {
                        "Desktop environment unknown"
                    } else {
                        &snapshot.host.desktop
                    },
                    snapshot.host.uptime
                ),
            ));
            root.append(&info_row(
                "CPU model",
                &snapshot.cpu.model,
                &format!("{} logical processors", snapshot.cpu.logical_cores),
            ));
            root.append(&label(
                "LIVE SYSTEM SNAPSHOT • AUTO REFRESH EVERY 3 SECONDS",
                "muted",
            ));
        }
    }

    root.upcast()
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--smart-scan") {
        print!("{}", linuxfo_core::collect_disk_health_helper_json());
        return;
    }

    let app = adw::Application::builder()
        .application_id("io.linuxfo.SystemInfo")
        .build();

    app.connect_activate(|app| {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(CSS);
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("LinuxFo")
            .default_width(1380)
            .default_height(900)
            .build();

        let header = gtk::HeaderBar::new();
        header.set_show_title_buttons(true);
        header.set_title_widget(Some(&gtk::Label::new(Some("LinuxFo"))));

        let style_manager = adw::StyleManager::default();
        if style_manager.is_dark() { window.add_css_class("dark"); }
        let window_for_theme = window.clone();
        style_manager.connect_notify_local(Some("dark"), move |manager, _| {
            if manager.is_dark() { window_for_theme.add_css_class("dark"); }
            else { window_for_theme.remove_css_class("dark"); }
        });
        let theme_button = gtk::MenuButton::new();
        theme_button.set_icon_name("applications-graphics-symbolic");
        theme_button.set_tooltip_text(Some("Appearance"));
        let theme_popover = gtk::Popover::new();
        let theme_box = gtk::Box::new(Orientation::Vertical, 4);
        theme_box.set_margin_top(8);
        theme_box.set_margin_bottom(8);
        theme_box.set_margin_start(8);
        theme_box.set_margin_end(8);
        let theme_system = gtk::CheckButton::with_label("Follow system");
        let theme_light = gtk::CheckButton::with_label("Light");
        let theme_dark = gtk::CheckButton::with_label("Dark");
        theme_light.set_group(Some(&theme_system));
        theme_dark.set_group(Some(&theme_system));
        match style_manager.color_scheme() {
            adw::ColorScheme::ForceLight => theme_light.set_active(true),
            adw::ColorScheme::ForceDark => theme_dark.set_active(true),
            _ => theme_system.set_active(true),
        }
        let manager = style_manager.clone();
        theme_system.connect_toggled(move |button| {
            if button.is_active() { manager.set_color_scheme(adw::ColorScheme::Default); }
        });
        let manager = style_manager.clone();
        theme_light.connect_toggled(move |button| {
            if button.is_active() { manager.set_color_scheme(adw::ColorScheme::ForceLight); }
        });
        let manager = style_manager.clone();
        theme_dark.connect_toggled(move |button| {
            if button.is_active() { manager.set_color_scheme(adw::ColorScheme::ForceDark); }
        });
        theme_box.append(&theme_system);
        theme_box.append(&theme_light);
        theme_box.append(&theme_dark);
        theme_popover.set_child(Some(&theme_box));
        theme_button.set_popover(Some(&theme_popover));
        header.pack_end(&theme_button);
        window.set_titlebar(Some(&header));

        let shell = gtk::Box::new(Orientation::Horizontal, 0);
        let sidebar = gtk::Box::new(Orientation::Vertical, 14);
        sidebar.set_width_request(224);
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(20);
        sidebar.set_margin_start(16);
        sidebar.set_margin_end(14);
        sidebar.add_css_class("sidebar");

        let brand_panel = gtk::Box::new(Orientation::Vertical, 8);
        brand_panel.add_css_class("brand-panel");
        let brand = gtk::Box::new(Orientation::Horizontal, 10);
        brand.set_valign(Align::Center);
        // Load the actual SVG file directly: icon-theme lookup is not reliable
        // on desktops whose icon cache has not been refreshed yet.
        let source_logo = concat!(env!("CARGO_MANIFEST_DIR"), "/../../packaging/io.linuxfo.SystemInfo.svg");
        let installed_logo = "/usr/share/icons/hicolor/scalable/apps/io.linuxfo.SystemInfo.svg";
        let logo_path = if std::path::Path::new(source_logo).is_file() {
            source_logo
        } else {
            installed_logo
        };
        let logo = gtk::Image::from_file(logo_path);
        logo.set_pixel_size(42);
        logo.set_size_request(42, 42);
        logo.set_valign(Align::Center);
        logo.add_css_class("brand-logo");
        brand.append(&logo);
        brand.append(&label("LinuxFo", "brand"));
        brand_panel.append(&brand);
        brand_panel.append(&label("YOUR SYSTEM, IN FOCUS", "muted"));
        sidebar.append(&brand_panel);

        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack.set_valign(Align::Fill);
        stack.set_transition_type(gtk::StackTransitionType::Crossfade);

        let pages = [
            "Overview", "CPU", "Memory", "Storage", "Disk Health", "Thermals", "Cameras", "Devices", "Network", "Processes", "Drivers", "Journal",
        ];

        let buttons = gtk::Box::new(Orientation::Vertical, 5);
        buttons.add_css_class("nav-section");
        let first = gtk::ToggleButton::new();
        let first_row = gtk::Box::new(Orientation::Horizontal, 10);
        first_row.append(&label("⌂", "nav-icon"));
        first_row.append(&label("Overview", "nav-label"));
        first.set_child(Some(&first_row));
        first.set_active(true);
        first.add_css_class("nav-button");
        buttons.append(&first);

        for page in pages.iter().skip(1) {
            let button = gtk::ToggleButton::new();
            let symbol = match *page {
                "CPU" => "◉",
                "Memory" => "▤",
                "Storage" => "▣",
                "Disk Health" => "♥",
                "Thermals" => "⌁",
                "Cameras" => "▣",
                "Devices" => "◇",
                "Processes" => "▥",
                "Drivers" => "⚙",
                "Journal" => "≡",
                _ => "⌘",
            };
            let row = gtk::Box::new(Orientation::Horizontal, 10);
            row.append(&label(symbol, "nav-icon"));
            row.append(&label(page, "nav-label"));
            button.set_child(Some(&row));
            button.set_group(Some(&first));
            button.add_css_class("nav-button");

            let stack_ref = stack.clone();
            let page_name = *page;
            button.connect_toggled(move |button| {
                if button.is_active() {
                    stack_ref.set_visible_child_name(page_name);
                    reset_visible_page_scroll(&stack_ref);
                }
            });
            buttons.append(&button);
        }

        let stack_ref = stack.clone();
        first.connect_toggled(move |button| {
            if button.is_active() {
                stack_ref.set_visible_child_name("Overview");
                reset_visible_page_scroll(&stack_ref);
            }
        });

        sidebar.append(&buttons);
        sidebar.append(&gtk::Separator::new(Orientation::Horizontal));
        sidebar.append(&label("LINUXFO  •  0.2.0", "muted"));

        let main_area = gtk::Box::new(Orientation::Vertical, 0);
        main_area.set_hexpand(true);

        let monitor = Rc::new(RefCell::new(System::new_all()));
        let snapshot = Rc::new(RefCell::new(collect_with_system(&mut monitor.borrow_mut())));
        let process_filter = Rc::new(RefCell::new(String::new()));
        for page in pages {
            let widget = content_page(&snapshot.borrow(), page, monitor.clone(), process_filter.clone());
            let page_scroller = page_scroller(&widget);
            stack.add_named(&page_scroller, Some(page));
        }

        main_area.append(&stack);

        shell.append(&sidebar);
        shell.append(&main_area);
        window.set_child(Some(&shell));
        window.present();

        let snapshot_ref = snapshot.clone();
        let monitor_ref = monitor.clone();
        let stack_ref = stack.clone();
        glib::timeout_add_local(Duration::from_secs(3), move || {
            *snapshot_ref.borrow_mut() = collect_with_system(&mut monitor_ref.borrow_mut());
            let page = stack_ref
                .visible_child_name()
                .map(|name| name.to_string())
                .unwrap_or_else(|| "Overview".into());
            if page == "Journal" || page == "Drivers" { return glib::ControlFlow::Continue; }
            let widget = content_page(&snapshot_ref.borrow(), &page, monitor_ref.clone(), process_filter.clone());
            let page_scroller = page_scroller(&widget);
            if let Some(old_child) = stack_ref.visible_child() {
                stack_ref.remove(&old_child);
            }
            stack_ref.add_named(&page_scroller, Some(&page));
            stack_ref.set_visible_child_name(&page);
            glib::ControlFlow::Continue
        });
    });

    app.run();
}

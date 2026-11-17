use adw::prelude::*;
use gtk::{glib, Align, Orientation};
use linuxfo_core::{collect_with_system, format_bytes, Snapshot};
use sysinfo::System;
use std::{cell::RefCell, rc::Rc, time::Duration};

const CSS: &str = r#"
window { background: #11151d; color: #edf2f7; }
headerbar { background: #171d27; border-bottom: 1px solid #293342; min-height: 48px; }
.sidebar { background: #171d27; border-right: 1px solid #293342; }
.brand { font-size: 22px; font-weight: 800; letter-spacing: -0.8px; color: #f4f7fb; }
.brand-mark { background: linear-gradient(135deg,#28d7c1,#6d5dfc); color: #07131b; border-radius: 12px; padding: 8px; font-weight: 900; }
.nav-button { background: transparent; color: #aab6c6; border-radius: 10px; padding: 10px 12px; text-align: left; }
.nav-button:hover,.nav-button:checked { background: #26313e; color: #fff; }
.page-title { font-size: 29px; font-weight: 750; letter-spacing: -0.7px; }
.muted { color: #91a0b2; }
.card { background: #1b2430; border: 1px solid #303d4d; border-radius: 16px; padding: 18px; }
.card-title { color: #9eafc1; font-size: 12px; font-weight: 700; letter-spacing: 0.7px; }
.metric { font-size: 28px; font-weight: 750; color: #f5f7fb; }
.metric-small { font-size: 15px; font-weight: 650; color: #f5f7fb; }
.accent { color: #40ddc4; font-weight: 700; }
.pill { background: #183c39; color: #70ead8; border-radius: 20px; padding: 5px 9px; font-size: 11px; }
boxedlist, list { background: #1b2430; border: 1px solid #303d4d; border-radius: 12px; }
list row { border-bottom: 1px solid #303d4d; }
separator { background: #303d4d; }
scrolledwindow { background: transparent; }
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

fn content_page(snapshot: &Snapshot, page: &str) -> gtk::Widget {
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
                    root.append(&info_row(&format!("{} • {}", disk.device, disk.model), &disk.status, &format!("{} • {}", disk.kind, extra)));
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
            let summary = format!("{} processes • sorted by CPU usage • refreshes every 3 seconds", snapshot.processes.len());
            root.append(&label(&summary, "muted"));
            let list = gtk::ListBox::new();
            list.add_css_class("boxed-list");
            for process in &snapshot.processes {
                let row = gtk::Box::new(Orientation::Horizontal, 12);
                row.set_margin_top(7);
                row.set_margin_bottom(7);
                row.set_margin_start(12);
                row.set_margin_end(12);
                let identity = gtk::Box::new(Orientation::Vertical, 3);
                identity.set_hexpand(true);
                identity.append(&label(&process.name, "metric-small"));
                identity.append(&label(&format!("PID {} • {}", process.pid, process.status), "muted"));
                row.append(&identity);
                row.append(&label(&format!("{:.1}%", process.cpu_percent), "accent"));
                row.append(&label(&format_bytes(process.memory_bytes), "muted"));
                let item = gtk::ListBoxRow::new();
                item.set_child(Some(&row));
                list.append(&item);
            }
            root.append(&list);
        }
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
        window.set_titlebar(Some(&header));

        let shell = gtk::Box::new(Orientation::Horizontal, 0);
        let sidebar = gtk::Box::new(Orientation::Vertical, 14);
        sidebar.set_width_request(224);
        sidebar.set_margin_top(24);
        sidebar.set_margin_bottom(20);
        sidebar.set_margin_start(16);
        sidebar.set_margin_end(14);
        sidebar.add_css_class("sidebar");

        let brand = gtk::Box::new(Orientation::Horizontal, 10);
        brand.append(&label("LF", "brand-mark"));
        brand.append(&label("LinuxFo", "brand"));
        sidebar.append(&brand);
        sidebar.append(&label("YOUR SYSTEM, IN FOCUS", "muted"));

        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack.set_transition_type(gtk::StackTransitionType::Crossfade);

        let scroller = gtk::ScrolledWindow::new();
        scroller.set_vexpand(true);
        scroller.set_hexpand(true);
        scroller.set_child(Some(&stack));

        let pages = [
            "Overview", "CPU", "Memory", "Storage", "Disk Health", "Thermals", "Cameras", "Devices", "Network", "Processes",
        ];

        let buttons = gtk::Box::new(Orientation::Vertical, 4);
        let first = gtk::CheckButton::new();
        first.set_label(Some("◫   Overview"));
        first.set_active(true);
        first.add_css_class("nav-button");
        buttons.append(&first);

        for page in pages.iter().skip(1) {
            let button = gtk::CheckButton::new();
            let symbol = match *page {
                "CPU" => "◉",
                "Memory" => "▤",
                "Storage" => "▣",
                "Disk Health" => "♥",
                "Thermals" => "⌁",
                "Cameras" => "▣",
                "Devices" => "◈",
                "Processes" => "▥",
                _ => "⌘",
            };
            button.set_label(Some(&format!("{symbol}   {page}")));
            button.set_group(Some(&first));
            button.add_css_class("nav-button");

            let stack_ref = stack.clone();
            let scroller_ref = scroller.clone();
            let page_name = *page;
            button.connect_toggled(move |button| {
                if button.is_active() {
                    stack_ref.set_visible_child_name(page_name);
                    scroller_ref.vadjustment().set_value(0.0);
                }
            });
            buttons.append(&button);
        }

        let stack_ref = stack.clone();
        let scroller_ref = scroller.clone();
        first.connect_toggled(move |button| {
            if button.is_active() {
                stack_ref.set_visible_child_name("Overview");
                scroller_ref.vadjustment().set_value(0.0);
            }
        });

        sidebar.append(&buttons);
        sidebar.append(&gtk::Separator::new(Orientation::Horizontal));
        sidebar.append(&label("LINUXFO  •  0.1.0", "muted"));

        let main_area = gtk::Box::new(Orientation::Vertical, 0);
        main_area.set_hexpand(true);

        let monitor = Rc::new(RefCell::new(System::new_all()));
        let snapshot = Rc::new(RefCell::new(collect_with_system(&mut monitor.borrow_mut())));
        for page in pages {
            let widget = content_page(&snapshot.borrow(), page);
            stack.add_named(&widget, Some(page));
        }

        main_area.append(&scroller);

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
            let widget = content_page(&snapshot_ref.borrow(), &page);
            if let Some(old_child) = stack_ref.visible_child() {
                stack_ref.remove(&old_child);
            }
            stack_ref.add_named(&widget, Some(&page));
            stack_ref.set_visible_child_name(&page);
            glib::ControlFlow::Continue
        });
    });

    app.run();
}

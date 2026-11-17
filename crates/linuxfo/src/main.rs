use adw::prelude::*;
use gtk::{glib, Align, Orientation};
use linuxfo_core::{collect_with_system, format_bytes, Snapshot};
use sysinfo::System;
use std::{cell::RefCell, rc::Rc, time::Duration};

const CSS: &str = r#"
* { font-family: "Inter", "Noto Sans", "Cantarell", sans-serif; }
window { background: #e9ebe8; color: #252a26; }
headerbar { background: #e5e8e3; color: #252a26; border-bottom: 1px solid alpha(currentColor, 0.10); min-height: 54px; }
.sidebar { background: #e5e8e3; border-right: 1px solid alpha(currentColor, 0.10); }
.brand { font-size: 23px; font-weight: 800; letter-spacing: -0.9px; color: #202621; }
.brand-mark { background: #343d36; color: #f4f5f1; border-radius: 11px; padding: 8px 9px; font-size: 13px; font-weight: 800; }
.nav-button { background: transparent; color: #606a62; border: 1px solid transparent; border-radius: 10px; padding: 10px 12px; min-height: 20px; font-size: 13px; font-weight: 550; text-align: left; }
.nav-button:hover { background: alpha(currentColor, 0.06); color: #252d27; }
.nav-button:checked { background: #dce8dd; color: #28633a; border: 1px solid #d0dfd2; font-weight: 700; }
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
window.dark headerbar, window.dark .sidebar { background: #252b26; color: #e5e9e4; }
window.dark .sidebar { border-right-color: #39413a; }
window.dark .brand, window.dark .page-title, window.dark .metric { color: #edf1eb; }
window.dark .brand-mark { background: #dce8dc; color: #263229; }
window.dark .nav-button { color: #b7c0b8; }
window.dark .nav-button:hover { background: #303a32; color: #ffffff; }
window.dark .nav-button:checked { background: #314637; color: #bce3c3; border-color: #405947; }
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

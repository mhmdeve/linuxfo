use adw::prelude::*;
use gtk::{glib, Align, Orientation};
use linuxfo_core::{collect, format_bytes, Snapshot};
use std::{cell::RefCell, rc::Rc, time::Duration};

const CSS: &str = r#"
window { background: #0b1020; color: #e8edf8; }
.sidebar { background: #10182b; border-right: 1px solid #24304a; }
.brand { font-size: 23px; font-weight: 800; letter-spacing: -0.8px; color: #f4f7ff; }
.brand-mark { background: linear-gradient(135deg,#6d7cff,#43d6c5); color: #08101e; border-radius: 12px; padding: 8px; font-weight: 900; }
.nav-button { background: transparent; color: #a9b5cc; border-radius: 10px; padding: 10px 12px; text-align: left; }
.nav-button:hover,.nav-button:checked { background: #202d48; color: #fff; }
.page-title { font-size: 29px; font-weight: 750; letter-spacing: -0.7px; }
.muted { color: #8d9bb6; }
.card { background: #141f34; border: 1px solid #263650; border-radius: 16px; padding: 18px; }
.card-title { color: #a9b8d1; font-size: 12px; font-weight: 700; letter-spacing: 0.7px; }
.metric { font-size: 28px; font-weight: 750; color: #f5f7ff; }
.metric-small { font-size: 17px; font-weight: 650; color: #f5f7ff; }
.accent { color: #69d8c9; }
.pill { background: #1d3b43; color: #7de5d4; border-radius: 20px; padding: 5px 9px; font-size: 11px; }
separator { background: #263650; }
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
        "Thermals" => ("Thermals & Sensors", "Kernel-exposed temperature and telemetry channels"),
        "Cameras" => ("Cameras", "Video devices discovered without activating streams"),
        "Devices" => ("Hardware Devices", "USB and PCI device inventory"),
        "Network" => ("Network", "Interface traffic counters"),
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

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("LinuxFo")
            .default_width(1240)
            .default_height(820)
            .build();

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

        let pages = [
            "Overview", "CPU", "Memory", "Storage", "Thermals", "Cameras", "Devices", "Network",
        ];

        let buttons = gtk::Box::new(Orientation::Vertical, 4);
        let first = gtk::CheckButton::new();
        first.set_label("◫   Overview");
        first.set_active(true);
        first.add_css_class("nav-button");
        buttons.append(&first);

        for page in pages.iter().skip(1) {
            let button = gtk::CheckButton::new();
            let symbol = match *page {
                "CPU" => "◉",
                "Memory" => "▤",
                "Storage" => "▣",
                "Thermals" => "⌁",
                "Cameras" => "▣",
                "Devices" => "◈",
                _ => "⌘",
            };
            button.set_label(&format!("{symbol}   {page}"));
            button.set_group(Some(&first));
            button.add_css_class("nav-button");

            let stack_ref = stack.clone();
            button.connect_toggled(move |button| {
                if button.is_active() {
                    stack_ref.set_visible_child_name(page);
                }
            });
            buttons.append(&button);
        }

        let stack_ref = stack.clone();
        first.connect_toggled(move |button| {
            if button.is_active() {
                stack_ref.set_visible_child_name("Overview");
            }
        });

        sidebar.append(&buttons);
        sidebar.append(&gtk::Separator::new(Orientation::Horizontal));
        sidebar.append(&label("LINUXFO  •  0.1.0", "muted"));

        let main_area = gtk::Box::new(Orientation::Vertical, 0);
        main_area.set_hexpand(true);

        let top_bar = gtk::Box::new(Orientation::Horizontal, 12);
        top_bar.set_margin_top(14);
        top_bar.set_margin_bottom(10);
        top_bar.set_margin_start(30);
        top_bar.set_margin_end(30);
        top_bar.append(&label("SYSTEM INTELLIGENCE", "muted"));
        top_bar.append(&label("●  LIVE", "pill"));
        top_bar.set_halign(Align::Fill);
        main_area.append(&top_bar);

        let snapshot = Rc::new(RefCell::new(collect()));
        for page in pages {
            let widget = content_page(&snapshot.borrow(), page);
            stack.add_named(&widget, Some(page));
        }

        let scroller = gtk::ScrolledWindow::new();
        scroller.set_vexpand(true);
        scroller.set_hexpand(true);
        scroller.set_child(Some(&stack));
        main_area.append(&scroller);

        shell.append(&sidebar);
        shell.append(&main_area);
        window.set_content(Some(&shell));
        window.present();

        let snapshot_ref = snapshot.clone();
        let stack_ref = stack.clone();
        glib::timeout_add_local(Duration::from_secs(3), move || {
            *snapshot_ref.borrow_mut() = collect();
            let page = stack_ref
                .visible_child_name()
                .map(|name| name.to_string())
                .unwrap_or_else(|| "Overview".into());
            let widget = content_page(&snapshot_ref.borrow(), &page);
            stack_ref.set_visible_child(&widget);
            glib::ControlFlow::Continue
        });
    });

    app.run();
}

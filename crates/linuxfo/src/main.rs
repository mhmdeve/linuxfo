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
    use std::collections::VecDeque;
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use std::sync::mpsc;

    let root = gtk::Box::new(Orientation::Vertical, 12);
    root.append(&section_header("System Journal", "Live systemd journal stream • newest entries load automatically"));

    let controls = gtk::Box::new(Orientation::Horizontal, 8);
    let severity = gtk::DropDown::from_strings(&["All", "Critical", "Error", "Warning", "Notice", "Info", "Debug"]);
    severity.set_tooltip_text(Some("Show this priority and more severe messages"));
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Filter message text…"));
    search.set_hexpand(true);
    let pause = gtk::Button::with_label("Pause");
    let clear = gtk::Button::with_label("Clear");
    controls.append(&severity);
    controls.append(&search);
    controls.append(&pause);
    controls.append(&clear);
    root.append(&controls);

    let status = label("Connecting to journalctl…", "muted");
    root.append(&status);

    let output = gtk::TextView::new();
    output.set_editable(false);
    output.set_cursor_visible(false);
    output.set_monospace(true);
    output.set_wrap_mode(gtk::WrapMode::WordChar);
    output.set_vexpand(true);
    let buffer = output.buffer();
    let log_scroll = gtk::ScrolledWindow::new();
    log_scroll.set_min_content_height(420);
    log_scroll.set_vexpand(true);
    log_scroll.set_hexpand(true);
    log_scroll.set_child(Some(&output));
    root.append(&log_scroll);

    // journalctl runs off the GTK thread. JSON output preserves PRIORITY and MESSAGE
    // so severity and text filters can be changed without restarting the stream.
    let (sender, receiver) = mpsc::channel::<(u8, String)>();
    std::thread::spawn(move || {
        let mut child = match Command::new("journalctl")
            .args(["--follow", "--lines=300", "--no-pager", "--output=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                let _ = sender.send((3, format!("LinuxFo: could not start journalctl: {error}")));
                return;
            }
        };
        if let Some(stdout) = child.stdout.take() {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break; };
                let parsed: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                let priority = parsed.get("PRIORITY")
                    .and_then(|value| value.as_str().and_then(|v| v.parse::<u8>().ok())
                        .or_else(|| value.as_u64().map(|v| v as u8)))
                    .unwrap_or(6);
                let message = parsed.get("MESSAGE")
                    .and_then(|value| value.as_str())
                    .unwrap_or("(journal entry without MESSAGE)");
                let timestamp = parsed.get("__REALTIME_TIMESTAMP")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                let identifier = parsed.get("SYSLOG_IDENTIFIER")
                    .and_then(|value| value.as_str())
                    .or_else(|| parsed.get("_COMM").and_then(|value| value.as_str()))
                    .unwrap_or("system");
                let rendered = if timestamp.is_empty() {
                    format!("[{identifier}] {message}")
                } else {
                    format!("[{timestamp}] [{identifier}] {message}")
                };
                if sender.send((priority, rendered)).is_err() { break; }
            }
        }
        let _ = child.kill();
        let _ = child.wait();
    });

    let entries = Rc::new(RefCell::new(VecDeque::<(u8, String)>::new()));
    let paused = Rc::new(RefCell::new(false));
    let pause_ref = paused.clone();
    let pause_button = pause.clone();
    pause.connect_clicked(move |_| {
        let mut is_paused = pause_ref.borrow_mut();
        *is_paused = !*is_paused;
        pause_button.set_label(if *is_paused { "Resume" } else { "Pause" });
    });

    let entries_for_clear = entries.clone();
    let buffer_for_clear = buffer.clone();
    clear.connect_clicked(move |_| {
        entries_for_clear.borrow_mut().clear();
        buffer_for_clear.set_text("");
    });

    let severity_ref = severity.clone();
    let search_ref = search.clone();
    let entries_ref = entries.clone();
    let buffer_ref = buffer.clone();
    let status_ref = status.clone();
    let scroll_ref = log_scroll.clone();
    let paused_ref = paused.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let mut received = false;
        if !*paused_ref.borrow() {
            while let Ok(entry) = receiver.try_recv() {
                let mut stored = entries_ref.borrow_mut();
                stored.push_back(entry);
                while stored.len() > 1500 { stored.pop_front(); }
                received = true;
            }
        } else {
            // Drain the pipe while paused so the background journalctl process
            // cannot block; resume shows only messages received after resuming.
            while receiver.try_recv().is_ok() {}
        }

        let priority_limit = match severity_ref.selected() {
            1 => Some(2), 2 => Some(3), 3 => Some(4),
            4 => Some(5), 5 => Some(6), 6 => Some(7), _ => None,
        };
        let needle = search_ref.text().to_string().to_lowercase();
        let should_render = received;
        if should_render {
            let lines: Vec<String> = entries_ref.borrow().iter()
                .filter(|(priority, _)| priority_limit.map(|limit| *priority <= limit).unwrap_or(true))
                .filter(|(_, line)| needle.is_empty() || line.to_lowercase().contains(&needle))
                .map(|(_, line)| line.clone())
                .collect();
            buffer_ref.set_text(&if lines.is_empty() {
                "No journal entries match the current filters.".into()
            } else { lines.join("\n") });
            let adjustment = scroll_ref.vadjustment();
            adjustment.set_value((adjustment.upper() - adjustment.page_size()).max(0.0));
        }
        let count = entries_ref.borrow().len();
        status_ref.set_text(&format!(
            "{} • {} buffered entries • {}",
            if *paused_ref.borrow() { "Paused" } else { "Live" },
            count,
            if count == 0 { "Waiting for journal entries" } else { "journalctl --follow" }
        ));
        glib::ControlFlow::Continue
    });

    // Re-render immediately when either filter changes, even if no new log line arrives.
    let entries_filter = entries.clone();
    let buffer_filter = buffer.clone();
    let severity_filter = severity.clone();
    let search_filter = search.clone();
    let redraw = Rc::new(move || {
        let priority_limit = match severity_filter.selected() {
            1 => Some(2), 2 => Some(3), 3 => Some(4),
            4 => Some(5), 5 => Some(6), 6 => Some(7), _ => None,
        };
        let needle = search_filter.text().to_string().to_lowercase();
        let lines: Vec<String> = entries_filter.borrow().iter()
            .filter(|(priority, _)| priority_limit.map(|limit| *priority <= limit).unwrap_or(true))
            .filter(|(_, line)| needle.is_empty() || line.to_lowercase().contains(&needle))
            .map(|(_, line)| line.clone())
            .collect();
        buffer_filter.set_text(&if lines.is_empty() {
            "No journal entries match the current filters.".into()
        } else { lines.join("\n") });
    });
    let redraw_severity = redraw.clone();
    severity.connect_selected_notify(move |_| redraw_severity());
    search.connect_search_changed(move |_| redraw());

    root.append(&label("Access: if system logs are unavailable, add your account to the systemd-journal group and sign in again. LinuxFo never runs the entire app as root.", "muted"));
    root.upcast()
}

fn performance_page(monitor: Rc<RefCell<System>>) -> gtk::Widget {
    use std::collections::VecDeque;

    let root = gtk::Box::new(Orientation::Vertical, 16);
    root.set_margin_top(28);
    root.set_margin_bottom(28);
    root.set_margin_start(30);
    root.set_margin_end(30);

    let cpu_model = {
        let system = monitor.borrow();
        system.cpus().first().map(|cpu| cpu.brand().to_string())
            .filter(|name| !name.is_empty()).unwrap_or_else(|| "Processor".into())
    };
    root.append(&section_header("Performance", &cpu_model));

    let summary = gtk::Grid::new();
    summary.set_column_spacing(12);
    summary.set_row_spacing(12);
    summary.set_column_homogeneous(true);

    let cpu_count = monitor.borrow().cpus().len();
    let histories: Vec<Rc<RefCell<VecDeque<f64>>>> = (0..cpu_count)
        .map(|_| Rc::new(RefCell::new(VecDeque::from(vec![0.0; 60]))))
        .collect();

    for (index, history) in histories.iter().enumerate() {
        let card = gtk::Box::new(Orientation::Vertical, 7);
        card.add_css_class("card");
        let heading = gtk::Box::new(Orientation::Horizontal, 6);
        heading.set_hexpand(true);
        heading.append(&label(&format!("CPU {index}"), "metric-small"));
        let value = label("0%", "accent");
        heading.append(&value);
        card.append(&heading);

        let chart = gtk::DrawingArea::new();
        chart.set_content_width(150);
        chart.set_content_height(76);
        chart.set_hexpand(true);
        let history_draw = history.clone();
        chart.set_draw_func(move |_, context, width, height| {
            let width = width as f64;
            let height = height as f64;
            context.set_source_rgba(0.22, 0.52, 0.88, 0.10);
            let _ = context.paint();
            let values = history_draw.borrow();
            if values.len() < 2 { return; }
            let step = width / (values.len() - 1) as f64;
            context.move_to(0.0, height);
            for (i, sample) in values.iter().enumerate() {
                let x = i as f64 * step;
                let y = height - (sample.clamp(0.0, 100.0) / 100.0 * (height - 4.0)) - 2.0;
                context.line_to(x, y);
            }
            context.line_to(width, height);
            context.close_path();
            context.set_source_rgba(0.20, 0.48, 0.84, 0.30);
            let _ = context.fill_preserve();
            context.set_source_rgb(0.30, 0.62, 0.96);
            context.set_line_width(1.5);
            let _ = context.stroke();
        });
        card.append(&chart);
        summary.attach(&card, (index % 4) as i32, (index / 4) as i32, 1, 1);

        let value_ref = value.clone();
        let history_ref = history.clone();
        let monitor_ref = monitor.clone();
        glib::timeout_add_local(Duration::from_secs(1), move || {
            let usage = monitor_ref.borrow().cpus().get(index)
                .map(|cpu| cpu.cpu_usage() as f64).unwrap_or(0.0);
            {
                let mut samples = history_ref.borrow_mut();
                samples.push_back(usage);
                while samples.len() > 60 { samples.pop_front(); }
            }
            value_ref.set_text(&format!("{usage:.0}%"));
            chart.queue_draw();
            glib::ControlFlow::Continue
        });
    }
    root.append(&summary);

    // Aggregate interface throughput, sampled directly from kernel sysfs counters.
    let network_card = gtk::Box::new(Orientation::Vertical, 10);
    network_card.add_css_class("card");
    network_card.append(&label("NETWORK TRAFFIC", "card-title"));
    let network_values = gtk::Grid::new();
    network_values.set_column_spacing(28);
    let download_value = label("↓ 0 B/s", "metric");
    let upload_value = label("↑ 0 B/s", "metric");
    let download_box = gtk::Box::new(Orientation::Vertical, 3);
    download_box.append(&label("DOWNLOAD", "card-title"));
    download_box.append(&download_value);
    let upload_box = gtk::Box::new(Orientation::Vertical, 3);
    upload_box.append(&label("UPLOAD", "card-title"));
    upload_box.append(&upload_value);
    network_values.attach(&download_box, 0, 0, 1, 1);
    network_values.attach(&upload_box, 1, 0, 1, 1);
    network_card.append(&network_values);
    let network_history = Rc::new(RefCell::new(VecDeque::<(f64, f64)>::from(vec![(0.0, 0.0); 60])));
    let network_chart = gtk::DrawingArea::new();
    network_chart.set_content_width(500);
    network_chart.set_content_height(90);
    network_chart.set_hexpand(true);
    let draw_history = network_history.clone();
    network_chart.set_draw_func(move |_, context, width, height| {
        let samples = draw_history.borrow();
        if samples.len() < 2 { return; }
        let width = width as f64;
        let height = height as f64;
        let max_rate = samples.iter().flat_map(|(rx, tx)| [*rx, *tx]).fold(1.0_f64, f64::max);
        let step = width / (samples.len() - 1) as f64;
        for (series, receive) in [(0_u8, true), (1_u8, false)] {
            context.new_path();
            for (i, (rx, tx)) in samples.iter().enumerate() {
                let rate = if receive { *rx } else { *tx };
                let x = i as f64 * step;
                let y = height - (rate / max_rate * (height - 8.0)) - 4.0;
                if i == 0 { context.move_to(x, y); } else { context.line_to(x, y); }
            }
            if series == 0 { context.set_source_rgb(0.30, 0.62, 0.96); }
            else { context.set_source_rgb(0.90, 0.50, 0.32); }
            context.set_line_width(2.0);
            let _ = context.stroke();
        }
    });
    network_card.append(&network_chart);
    network_card.append(&label("Combined interface throughput • 60-second history • excludes loopback", "muted"));
    root.append(&network_card);

    let previous = Rc::new(RefCell::new(None::<(u64, u64)>));
    let previous_ref = previous.clone();
    let history_ref = network_history.clone();
    let download_ref = download_value.clone();
    let upload_ref = upload_value.clone();
    let chart_ref = network_chart.clone();
    glib::timeout_add_local(Duration::from_secs(1), move || {
        let mut rx = 0_u64;
        let mut tx = 0_u64;
        if let Ok(interfaces) = std::fs::read_dir("/sys/class/net") {
            for interface in interfaces.flatten() {
                if interface.file_name() == "lo" { continue; }
                let stats = interface.path().join("statistics");
                rx = rx.saturating_add(sysfs_value(&stats.join("rx_bytes")).parse().unwrap_or(0));
                tx = tx.saturating_add(sysfs_value(&stats.join("tx_bytes")).parse().unwrap_or(0));
            }
        }
        let (rx_rate, tx_rate) = {
            let mut previous = previous_ref.borrow_mut();
            let rates = previous.map(|(old_rx, old_tx)| (
                rx.saturating_sub(old_rx) as f64,
                tx.saturating_sub(old_tx) as f64,
            )).unwrap_or((0.0, 0.0));
            *previous = Some((rx, tx));
            rates
        };
        {
            let mut samples = history_ref.borrow_mut();
            samples.push_back((rx_rate, tx_rate));
            while samples.len() > 60 { samples.pop_front(); }
        }
        download_ref.set_text(&format!("↓ {}/s", format_bytes(rx_rate as u64)));
        upload_ref.set_text(&format!("↑ {}/s", format_bytes(tx_rate as u64)));
        chart_ref.queue_draw();
        glib::ControlFlow::Continue
    });

    let memory_card = gtk::Box::new(Orientation::Vertical, 10);
    memory_card.add_css_class("card");
    memory_card.append(&label("MEMORY", "card-title"));
    let memory_value = label("Reading memory…", "metric");
    memory_card.append(&memory_value);
    let memory_detail = label("", "muted");
    memory_card.append(&memory_detail);
    let memory_bar = gtk::ProgressBar::new();
    memory_card.append(&memory_bar);

    let swap_card = gtk::Box::new(Orientation::Vertical, 10);
    swap_card.add_css_class("card");
    swap_card.append(&label("SWAP", "card-title"));
    let swap_value = label("Reading swap…", "metric");
    swap_card.append(&swap_value);
    let swap_detail = label("", "muted");
    swap_card.append(&swap_detail);
    let swap_bar = gtk::ProgressBar::new();
    swap_card.append(&swap_bar);

    let lower = gtk::Grid::new();
    lower.set_column_spacing(12);
    lower.set_column_homogeneous(true);
    lower.attach(&memory_card, 0, 0, 1, 1);
    lower.attach(&swap_card, 1, 0, 1, 1);
    root.append(&lower);

    let memory_value_ref = memory_value.clone();
    let memory_detail_ref = memory_detail.clone();
    let memory_bar_ref = memory_bar.clone();
    let swap_value_ref = swap_value.clone();
    let swap_detail_ref = swap_detail.clone();
    let swap_bar_ref = swap_bar.clone();
    let monitor_stats = monitor.clone();
    glib::timeout_add_local(Duration::from_secs(1), move || {
        let system = monitor_stats.borrow();
        let total = system.total_memory();
        let used = system.used_memory();
        let swap_total = system.total_swap();
        let swap_used = system.used_swap();
        let memory_percent = if total == 0 { 0.0 } else { used as f64 * 100.0 / total as f64 };
        let swap_percent = if swap_total == 0 { 0.0 } else { swap_used as f64 * 100.0 / swap_total as f64 };
        memory_value_ref.set_text(&format!("{:.1}%", memory_percent));
        memory_detail_ref.set_text(&format!("{} used • {} total • {} available",
            format_bytes(used), format_bytes(total), format_bytes(total.saturating_sub(used))));
        memory_bar_ref.set_fraction((memory_percent / 100.0).clamp(0.0, 1.0));
        swap_value_ref.set_text(&format!("{:.1}%", swap_percent));
        swap_detail_ref.set_text(&format!("{} used • {} total",
            format_bytes(swap_used), format_bytes(swap_total)));
        swap_bar_ref.set_fraction((swap_percent / 100.0).clamp(0.0, 1.0));
        glib::ControlFlow::Continue
    });

    root.append(&label("Per-logical-CPU utilisation history • 60-second rolling window • updates every second", "muted"));
    root.upcast()
}

fn content_page(snapshot: &Snapshot, page: &str, monitor: Rc<RefCell<System>>, process_filter: Rc<RefCell<String>>) -> gtk::Widget {
    let root = gtk::Box::new(Orientation::Vertical, 18);
    root.set_margin_top(28);
    root.set_margin_bottom(28);
    root.set_margin_start(30);
    root.set_margin_end(30);

    let (title, subtitle) = match page {
        "Performance" => ("Performance", "Per-core CPU, memory and swap utilisation"),
        "Storage" => ("Storage", "Filesystems, devices and mount points"),
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
        "Performance" => return performance_page(monitor.clone()),
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
            root.append(&section_header("Drive Health (SMART / NVMe)", "Drive condition, temperature, endurance and activity"));
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
                    let busy_text = disk.performance_percent.map(|v| format!("{:.0}%", v.clamp(0.0, 100.0))).unwrap_or_else(|| "Sampling…".into());
                    metrics_grid.attach(&label("HEALTH", "card-title"), 0, 0, 1, 1);
                    metrics_grid.attach(&label("DISK BUSY TIME", "card-title"), 1, 0, 1, 1);
                    metrics_grid.attach(&label(&health_text, "metric-small"), 0, 1, 1, 1);
                    metrics_grid.attach(&label(&busy_text, "metric-small"), 1, 1, 1, 1);
                    for (column, value) in [(0, disk.health_percent), (1, disk.performance_percent)] {
                        if let Some(value) = value {
                            let bar = gtk::ProgressBar::new();
                            bar.set_fraction(value.clamp(0.0, 100.0) / 100.0);
                            metrics_grid.attach(&bar, column, 2, 1, 1);
                        }
                    }
                    card.append(&metrics_grid);
                    let note = if disk.percentage_used.is_some() {
                        "NVMe health is derived from remaining rated endurance"
                    } else if disk.health_percent.is_some() {
                        "SMART pass/fail assessment, not remaining lifespan"
                    } else {
                        "Drive does not expose a usable SMART health percentage"
                    };
                    card.append(&label(&format!("{} • Busy time is not a speed score • {}", extra, note), "muted"));
                    root.append(&card);
                }
            }
        }

        "Storage" => {
            if snapshot.disks.is_empty() {
                root.append(&info_row("No mounted filesystems found", "—", "No mounted storage volumes were discovered."));
            }
            for disk in &snapshot.disks {
                let used = disk.total_bytes.saturating_sub(disk.available_bytes);
                let percent = if disk.total_bytes == 0 { 0.0 } else { used as f64 * 100.0 / disk.total_bytes as f64 };
                let card = gtk::Box::new(Orientation::Vertical, 8);
                card.add_css_class("card");
                card.append(&info_row(&disk.name, &format_bytes(disk.total_bytes), &format!(
                    "{} • {} • {} used • {} free{}",
                    disk.mount_point, disk.filesystem, format_bytes(used),
                    format_bytes(disk.available_bytes), if disk.removable { " • removable" } else { "" }
                )));
                let bar = gtk::ProgressBar::new();
                bar.set_fraction((percent / 100.0).clamp(0.0, 1.0));
                card.append(&bar);
                card.append(&label(&format!("USED SPACE • {percent:.1}%"), "muted"));
                root.append(&card);
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
            "Overview", "Performance", "Storage", "Thermals", "Cameras", "Devices", "Network", "Processes", "Drivers", "Journal",
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
                "Performance" => "▤",
                "Storage" => "▣",
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
            if page == "Journal" || page == "Drivers" || page == "Performance" { return glib::ControlFlow::Continue; }
            let previous_scroll = stack_ref.visible_child()
                .and_then(|child| child.downcast::<gtk::ScrolledWindow>().ok())
                .map(|scroller| scroller.vadjustment().value())
                .unwrap_or(0.0);
            let widget = content_page(&snapshot_ref.borrow(), &page, monitor_ref.clone(), process_filter.clone());
            let page_scroller = page_scroller(&widget);
            if let Some(old_child) = stack_ref.visible_child() {
                stack_ref.remove(&old_child);
            }
            stack_ref.add_named(&page_scroller, Some(&page));
            stack_ref.set_visible_child_name(&page);
            let stack_restore = stack_ref.clone();
            let page_restore = page.clone();
            glib::idle_add_local_once(move || {
                if let Some(scroller) = stack_restore.child_by_name(&page_restore)
                    .and_then(|child| child.downcast::<gtk::ScrolledWindow>().ok())
                {
                    let adjustment = scroller.vadjustment();
                    let max_value = (adjustment.upper() - adjustment.page_size()).max(0.0);
                    adjustment.set_value(previous_scroll.min(max_value));
                }
            });
            glib::ControlFlow::Continue
        });
    });

    app.run();
}

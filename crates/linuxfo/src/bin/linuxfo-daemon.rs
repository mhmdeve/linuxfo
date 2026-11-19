//! LinuxFo privileged system daemon.
//!
//! The daemon runs as root under systemd and exposes only a fixed, read-only
//! SMART/NVMe inventory operation over a local Unix socket. Never accept
//! executable paths, shell commands, arbitrary device paths, or user supplied
//! smartctl arguments from clients.

use linuxfo_core::collect_disk_health_helper_json;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::thread;

const SOCKET_PATH: &str = "/run/linuxfo/daemon.sock";

fn handle_client(mut stream: UnixStream) {
    let cloned = match stream.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            eprintln!("linuxfo-daemon: failed to clone client stream: {error}");
            return;
        }
    };
    let mut reader = BufReader::new(cloned);
    let mut request = String::new();
    if reader.read_line(&mut request).is_err() || request.len() > 4096 {
        let _ = stream.write_all(b"[]\n");
        return;
    }

    let action = serde_json::from_str::<serde_json::Value>(&request)
        .ok()
        .and_then(|value| value.get("action").and_then(|action| action.as_str()).map(str::to_owned));

    match action.as_deref() {
        Some("disk-health") => {
            let response = collect_disk_health_helper_json();
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(b"\n");
        }
        _ => {
            let _ = stream.write_all(b"[]\n");
        }
    }
}

fn main() -> std::io::Result<()> {
    let uid = fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| status.lines().find(|line| line.starts_with("Uid:")).map(str::to_owned))
        .and_then(|line| line.split_whitespace().nth(1).and_then(|value| value.parse::<u32>().ok()));
    if uid != Some(0) {
        eprintln!("linuxfo-daemon must run as root (systemd service configuration error)");
        std::process::exit(1);
    }

    let socket = Path::new(SOCKET_PATH);
    if let Some(parent) = socket.parent() {
        fs::create_dir_all(parent)?;
    }
    if socket.exists() {
        fs::remove_file(socket)?;
    }

    let listener = UnixListener::bind(socket)?;
    fs::set_permissions(socket, fs::Permissions::from_mode(0o666))?;
    eprintln!("linuxfo-daemon: listening on {SOCKET_PATH}");

    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                thread::spawn(|| handle_client(stream));
            }
            Err(error) => eprintln!("linuxfo-daemon: socket accept failed: {error}"),
        }
    }
    Ok(())
}

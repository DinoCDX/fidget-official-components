use super::{run, WindowInfo};
use x11rb::wrapper::ConnectionExt as _;
use anyhow::Result;
use launcher_core::{Action, SystemCommand};
use std::path::PathBuf;
use x11rb::{
    connection::Connection,
    protocol::xproto::{AtomEnum, ClientMessageEvent, ConnectionExt, EventMask},
    rust_connection::RustConnection,
};

fn atom(conn: &RustConnection, name: &str) -> Result<u32> {
    Ok(conn.intern_atom(false, name.as_bytes())?.reply()?.atom)
}

pub fn list_windows() -> Result<Vec<WindowInfo>> {
    let Ok((conn, screen_num)) = x11rb::connect(None) else {
        anyhow::bail!("window switching needs an X11 session, Wayland is not supported yet");
    };
    let root = conn.setup().roots[screen_num].root;
    let client_list = atom(&conn, "_NET_CLIENT_LIST")?;
    let net_wm_name = atom(&conn, "_NET_WM_NAME")?;
    let utf8 = atom(&conn, "UTF8_STRING")?;

    let list = conn
        .get_property(false, root, client_list, AtomEnum::WINDOW, 0, u32::MAX)?
        .reply()?;
    let Some(ids) = list.value32() else {
        return Ok(Vec::new());
    };
    Ok(ids
        .filter_map(|window| window_info(&conn, window, net_wm_name, utf8))
        .collect())
}

fn window_info(conn: &RustConnection, window: u32, net_wm_name: u32, utf8: u32) -> Option<WindowInfo> {
    let title = property(conn, window, net_wm_name, utf8)
        .or_else(|| property(conn, window, u32::from(AtomEnum::WM_NAME), u32::from(AtomEnum::STRING)))?;
    let title = String::from_utf8_lossy(&title).into_owned();
    if title.is_empty() {
        return None;
    }
    let class = property(conn, window, u32::from(AtomEnum::WM_CLASS), u32::from(AtomEnum::STRING))?;
    let app = class
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .next_back()
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .unwrap_or_default();
    Some(WindowInfo {
        handle: u64::from(window),
        title,
        app,
    })
}

fn property(conn: &RustConnection, window: u32, property: u32, kind: u32) -> Option<Vec<u8>> {
    let reply = conn
        .get_property(false, window, property, kind, 0, 1024)
        .ok()?
        .reply()
        .ok()?;
    if reply.value.is_empty() {
        None
    } else {
        Some(reply.value)
    }
}

pub fn focus(handle: u64, _app: &str) -> Result<()> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen_num].root;
    let active = atom(&conn, "_NET_ACTIVE_WINDOW")?;
    let event = ClientMessageEvent::new(32, handle as u32, active, [2u32, 0, 0, 0, 0]);
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        event,
    )?;
    conn.flush()?;
    Ok(())
}

pub fn system(command: SystemCommand) -> Result<()> {
    match command {
        SystemCommand::Lock => run("loginctl", ["lock-session"]),
        SystemCommand::Sleep => run("systemctl", ["suspend"]),
        SystemCommand::Restart => run("systemctl", ["reboot"]),
        SystemCommand::Shutdown => run("systemctl", ["poweroff"]),
        SystemCommand::EmptyRecycleBin => run("gio", ["trash", "--empty"]),
    }
}

pub fn index_apps() -> Vec<(String, Action)> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
    ];
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local/share/applications"));
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }

    let mut apps = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(app) = parse_desktop(&text) {
                apps.push(app);
            }
        }
    }
    apps.sort_by(|a, b| a.0.cmp(&b.0));
    apps.dedup_by(|a, b| a.0 == b.0);
    apps
}

fn parse_desktop(text: &str) -> Option<(String, Action)> {
    let mut name = None;
    let mut exec = None;
    let mut hidden = false;
    let mut is_app = false;
    let mut in_entry = false;

    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some(value) = line.strip_prefix("Name=") {
            name.get_or_insert_with(|| value.to_owned());
        } else if let Some(value) = line.strip_prefix("Exec=") {
            exec = Some(value.to_owned());
        } else if line == "NoDisplay=true" || line == "Hidden=true" {
            hidden = true;
        } else if line == "Type=Application" {
            is_app = true;
        }
    }

    if hidden || !is_app {
        return None;
    }
    let name: String = name?;
    let exec: String = exec?;
    let mut parts = exec.split_whitespace().filter(|part| !part.starts_with('%'));
    let program = PathBuf::from(parts.next()?);
    let args = parts.map(str::to_owned).collect();
    Some((name, Action::Launch { program, args }))
}

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use x11rb::protocol::xproto::PropMode;

pub fn native_backdrop<W: HasWindowHandle>(window: &W) -> bool {
    kde_blur(window).is_ok()
}

fn kde_blur<W: HasWindowHandle>(window: &W) -> Result<()> {
    let handle = window.window_handle()?;
    let id = match handle.as_raw() {
        RawWindowHandle::Xlib(raw) => raw.window as u32,
        RawWindowHandle::Xcb(raw) => raw.window.get(),
        _ => anyhow::bail!("native blur needs an X11 window"),
    };
    let (conn, _) = x11rb::connect(None)?;
    let property = atom(&conn, "_KDE_NET_WM_BLUR_BEHIND_REGION")?;
    conn.change_property32(PropMode::REPLACE, id, property, AtomEnum::CARDINAL, &[])?;
    conn.flush()?;
    Ok(())
}

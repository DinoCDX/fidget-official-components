use super::{run, WindowInfo};
use anyhow::{bail, Result};
use launcher_core::{Action, SystemCommand};
use std::path::PathBuf;
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{BOOL, CloseHandle, HWND, LPARAM},
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
            IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
        },
    },
};

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return BOOL(1);
    }
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length <= 0 {
        return BOOL(1);
    }
    let mut buffer = vec![0u16; length as usize + 1];
    let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    let title = String::from_utf16_lossy(&buffer[..copied.max(0) as usize]);
    if title.is_empty() {
        return BOOL(1);
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    windows.push(WindowInfo {
        handle: hwnd.0 as usize as u64,
        title,
        app: process_name(pid).unwrap_or_default(),
    });
    BOOL(1)
}

fn process_name(pid: u32) -> Option<String> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = vec![0u16; 1024];
        let mut size = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(process);
        result.ok()?;
        let path = PathBuf::from(String::from_utf16_lossy(&buffer[..size as usize]));
        path.file_stem().map(|stem| stem.to_string_lossy().into_owned())
    }
}

pub fn list_windows() -> Result<Vec<WindowInfo>> {
    let mut windows: Vec<WindowInfo> = Vec::new();
    unsafe {
        EnumWindows(Some(collect), LPARAM(&mut windows as *mut Vec<WindowInfo> as isize))?;
    }
    Ok(windows)
}

pub fn focus(handle: u64, _app: &str) -> Result<()> {
    let hwnd = HWND(handle as usize as *mut core::ffi::c_void);
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        if !SetForegroundWindow(hwnd).as_bool() {
            bail!("Windows refused to bring the window to the foreground");
        }
    }
    Ok(())
}

pub fn system(command: SystemCommand) -> Result<()> {
    match command {
        SystemCommand::Lock => run("rundll32.exe", ["user32.dll,LockWorkStation"]),
        SystemCommand::Sleep => run("rundll32.exe", ["powrprof.dll,SetSuspendState", "0,1,0"]),
        SystemCommand::Restart => run("shutdown", ["/r", "/t", "0"]),
        SystemCommand::Shutdown => run("shutdown", ["/s", "/t", "0"]),
        SystemCommand::EmptyRecycleBin => run(
            "powershell",
            [
                "-NoProfile",
                "-WindowStyle",
                "Hidden",
                "-Command",
                "Clear-RecycleBin -Force -ErrorAction SilentlyContinue",
            ],
        ),
    }
}

pub fn index_apps() -> Vec<(String, Action)> {
    let roots: Vec<PathBuf> = [
        std::env::var_os("ProgramData"),
        std::env::var_os("APPDATA"),
    ]
    .into_iter()
    .flatten()
    .map(|base| PathBuf::from(base).join(r"Microsoft\Windows\Start Menu\Programs"))
    .collect();

    let mut apps = Vec::new();
    for root in roots {
        for entry in walkdir::WalkDir::new(root).into_iter().filter_map(Result::ok) {
            let path = entry.into_path();
            let is_shortcut = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("lnk"));
            if !is_shortcut {
                continue;
            }
            if let Some(name) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) {
                apps.push((name, Action::Open(path)));
            }
        }
    }
    apps.sort_by(|a, b| a.0.cmp(&b.0));
    apps.dedup_by(|a, b| a.0 == b.0);
    apps
}

use raw_window_handle::HasWindowHandle;

pub fn native_backdrop<W: HasWindowHandle>(window: &W) -> bool {
    window_vibrancy::apply_mica(window, None).is_ok()
        || window_vibrancy::apply_acrylic(window, None).is_ok()
}

use launcher_core::IconPixels;
use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::Path};
use windows::{
    core::PCWSTR,
    Win32::{
        Graphics::Gdi::{
            DeleteObject, GetDC, GetDIBits, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER,
            DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
        },
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        UI::{
            Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON},
            WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO},
        },
    },
};

const ICON_SIZE: i32 = 32;

pub fn file_icon(path: &Path) -> Option<IconPixels> {
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let mut info: SHFILEINFOW = std::mem::zeroed();
        let found = SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if found == 0 || info.hIcon.is_invalid() {
            return None;
        }
        let icon = info.hIcon;
        let mut icon_info: ICONINFO = std::mem::zeroed();
        let pixels = if GetIconInfo(icon, &mut icon_info).is_ok() {
            read_colour_bitmap(icon_info.hbmColor)
        } else {
            None
        };
        if !icon_info.hbmColor.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(icon_info.hbmColor.0));
        }
        if !icon_info.hbmMask.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(icon_info.hbmMask.0));
        }
        let _ = DestroyIcon(icon);
        pixels
    }
}

unsafe fn read_colour_bitmap(bitmap: HBITMAP) -> Option<IconPixels> {
    let dc = GetDC(HWND::default());
    let mut info: BITMAPINFO = std::mem::zeroed();
    let header = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: ICON_SIZE,
        biHeight: -ICON_SIZE,
        biPlanes: 1,
        biBitCount: 32,
        ..std::mem::zeroed()
    };
    info.bmiHeader = header;
    let mut pixels = vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
    let lines = GetDIBits(
        dc,
        bitmap,
        0,
        ICON_SIZE as u32,
        Some(pixels.as_mut_ptr() as *mut c_void),
        &mut info,
        DIB_RGB_COLORS,
    );
    ReleaseDC(HWND::default(), dc);
    if lines != ICON_SIZE {
        return None;
    }
    for px in pixels.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    let has_alpha = pixels.chunks_exact(4).any(|px| px[3] != 0);
    if !has_alpha {
        for px in pixels.chunks_exact_mut(4) {
            px[3] = 255;
        }
    }
    Some(IconPixels {
        width: ICON_SIZE as u32,
        height: ICON_SIZE as u32,
        rgba: pixels,
    })
}

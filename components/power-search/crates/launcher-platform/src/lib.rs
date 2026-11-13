use anyhow::Result;
use launcher_settings::config::Blur;
use raw_window_handle::HasWindowHandle;
use std::{ffi::OsStr, path::Path, process::Command};

#[cfg(not(any(windows, target_os = "linux")))]
compile_error!("launcher supports Windows and Linux only");

mod actions;
#[cfg(windows)]
mod win;
#[cfg(target_os = "linux")]
mod linux;

pub use actions::execute;
#[cfg(windows)]
use win::native_backdrop;
#[cfg(target_os = "linux")]
use linux::native_backdrop;

#[cfg(windows)]
pub use win::{focus, index_apps, list_windows, system};
#[cfg(target_os = "linux")]
pub use linux::{focus, index_apps, list_windows, system};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub handle: u64,
    pub title: String,
    pub app: String,
}

#[cfg(windows)]
pub fn open_path(path: &Path) -> Result<()> {
    run("explorer", [path.as_os_str()])
}

#[cfg(target_os = "linux")]
pub fn open_path(path: &Path) -> Result<()> {
    run("xdg-open", [path.as_os_str()])
}

pub(crate) fn run<I, S>(program: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(program).args(args).spawn()?;
    Ok(())
}

pub fn apply_backdrop<W: HasWindowHandle>(window: &W, blur: Blur) -> bool {
    match blur {
        Blur::Off => false,
        Blur::Auto => native_backdrop(window),
    }
}

use launcher_core::IconPixels;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

type IconCache = Mutex<HashMap<PathBuf, Option<IconPixels>>>;

pub fn icon_for(path: &Path) -> Option<IconPixels> {
    static CACHE: OnceLock<IconCache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(map) = cache.lock() {
        if let Some(hit) = map.get(path) {
            return hit.clone();
        }
    }
    let icon = platform_icon(path);
    if let Ok(mut map) = cache.lock() {
        map.insert(path.to_path_buf(), icon.clone());
    }
    icon
}

#[cfg(windows)]
use win::file_icon as platform_icon;

#[cfg(target_os = "linux")]
fn platform_icon(_path: &Path) -> Option<IconPixels> {
    None
}

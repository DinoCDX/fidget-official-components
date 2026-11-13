mod app;
mod registry;
mod ui;
mod viewport;

use app::{config_path, Launcher, CHROME_HEIGHT};
use eframe::egui;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyManager};
use launcher_settings::Config;

const HELP: &str = "launcher - keyboard launcher

Usage: launcher [option]

  -h, --help      show this help
  -V, --version   print the version
      --config    print the config file path

Press the summon hotkey (Alt+Space by default) to open.";

fn main() -> eframe::Result {
    match std::env::args().nth(1).as_deref() {
        Some("-V") | Some("--version") => {
            println!("launcher {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some("-h") | Some("--help") => {
            println!("{HELP}");
            return Ok(());
        }
        Some("--config") => {
            println!("{}", config_path().display());
            return Ok(());
        }
        _ => {}
    }

    let path = config_path();
    let first_run = !path.exists();
    let mut notices = Vec::new();
    if first_run {
        notices.push(format!(
            "Welcome. Settings were created at {}. Press Alt+Space to search.",
            path.display()
        ));
    }
    let mut config = match Config::load(&path) {
        Ok(config) => config,
        Err(err) => {
            notices.push(format!("Config error, using defaults: {err}"));
            Config::default()
        }
    };
    if let Some(values) = fidget_values() {
        config.apply_overrides(&values);
    }

    if cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some() {
        notices.push(String::from(
            "Global hotkeys and window switching need X11 on Linux",
        ));
    }

    let (hotkeys, hotkey, hotkey_notice) = setup_hotkey(&config.hotkey.summon);
    notices.extend(hotkey_notice);
    let notice = (!notices.is_empty()).then(|| notices.join(" | "));

    let registry = registry::build(&config);
    registry.warm();

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([config.appearance.width, CHROME_HEIGHT])
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_visible(false)
        .with_resizable(false);
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Fidgetify",
        options,
        Box::new(move |_cc| {
            let app = Launcher::new(config, registry, hotkeys, hotkey, notice);
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
}

fn setup_hotkey(summon: &str) -> (Option<GlobalHotKeyManager>, Option<HotKey>, Option<String>) {
    let manager = match GlobalHotKeyManager::new() {
        Ok(manager) => manager,
        Err(err) => {
            return (
                None,
                None,
                Some(format!("Global hotkeys are unavailable: {err}")),
            )
        }
    };
    let hotkey: HotKey = match summon.parse() {
        Ok(hotkey) => hotkey,
        Err(err) => {
            return (
                Some(manager),
                None,
                Some(format!("Invalid hotkey \"{summon}\": {err}")),
            )
        }
    };
    match manager.register(hotkey) {
        Ok(()) => (Some(manager), Some(hotkey), None),
        Err(err) => (
            Some(manager),
            None,
            Some(format!(
                "Could not register {summon} ({err}). Another app or launcher may use it. Change hotkey.summon in {}",
                config_path().display()
            )),
        ),
    }
}

fn fidget_values() -> Option<toml::Table> {
    let path = std::env::var_os("FIDGET_VALUES")?;
    let text = std::fs::read_to_string(path).ok()?;
    toml::from_str(&text).ok()
}

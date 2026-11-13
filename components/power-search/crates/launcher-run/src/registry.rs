use launcher_core::Registry;
use launcher_mod_apps::Apps;
use launcher_mod_calc::Calculator;
use launcher_mod_files::Files;
use launcher_mod_system::Commands;
use launcher_mod_windows::Windows;
use launcher_settings::Config;
use std::sync::Arc;

pub fn build(config: &Config) -> Registry {
    let mut registry = Registry::new(config.priorities.clone());
    registry.register(Arc::new(Calculator::new(config.general.auto_calculate)));
    registry.register(Arc::new(Apps::default()));
    registry.register(Arc::new(Files::default()));
    registry.register(Arc::new(Windows));
    registry.register(Arc::new(Commands));
    registry
}

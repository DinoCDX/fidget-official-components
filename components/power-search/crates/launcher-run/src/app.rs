use crate::{registry, ui, viewport::Viewport};
use eframe::egui::{self, Key, Modifiers};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use launcher_core::{engine, Action, Cancel, Registry, ResultItem, Router, SearchEvent};
use launcher_settings::{config::Blur, Config, ResolvedTheme};
use notify::{RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::Ordering,
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};

pub const ROW_HEIGHT: f32 = 52.0;
pub const CHROME_HEIGHT: f32 = 104.0;
pub const HINTS_HEIGHT: f32 = 132.0;

const BLUR_GRACE: Duration = Duration::from_millis(400);
const DIGITS: [Key; 9] = [
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
];

pub fn config_path() -> PathBuf {
    Config::path().unwrap_or_else(|| PathBuf::from("config.toml"))
}

pub fn window_height(hints: bool, rows: usize) -> f32 {
    let body = if hints {
        HINTS_HEIGHT
    } else {
        rows as f32 * ROW_HEIGHT
    };
    CHROME_HEIGHT + body
}

pub fn icon_key(item: &ResultItem) -> String {
    format!("{}|{}", item.title, item.subtitle.as_deref().unwrap_or(""))
}

pub struct Launcher {
    pub config: Config,
    pub theme: ResolvedTheme,
    pub visible: bool,
    pub query: String,
    pub results: Vec<ResultItem>,
    pub viewport: Viewport,
    pub pending_confirm: Option<usize>,
    pub status: String,
    pub icons: HashMap<String, egui::TextureHandle>,
    pub focus_pending: bool,
    pub native_blur: bool,
    pub fade: f32,
    registry: Registry,
    router: Router,
    hotkeys: Option<GlobalHotKeyManager>,
    hotkey: Option<HotKey>,
    config_rx: Receiver<Config>,
    _watcher: Option<notify::RecommendedWatcher>,
    generation: u64,
    search: Option<(Receiver<SearchEvent>, Cancel)>,
    shown_at: Option<Instant>,
    startup_notice: bool,
    blur_checked: bool,
    last_size: (f32, f32),
}

impl Launcher {
    pub fn new(
        config: Config,
        registry: Registry,
        hotkeys: Option<GlobalHotKeyManager>,
        hotkey: Option<HotKey>,
        notice: Option<String>,
    ) -> Self {
        let (tx, config_rx) = mpsc::channel();
        let watcher = watch_config(tx);
        let theme = resolve_theme(&config);
        let router = Router::new(&config.prefixes);
        let viewport = Viewport::new(config.general.max_visible_results);
        Self {
            config,
            theme,
            visible: false,
            query: String::new(),
            results: Vec::new(),
            viewport,
            pending_confirm: None,
            status: notice.clone().unwrap_or_else(|| String::from("Ready")),
            icons: HashMap::new(),
            focus_pending: false,
            native_blur: false,
            fade: 1.0,
            registry,
            router,
            hotkeys,
            hotkey,
            config_rx,
            _watcher: watcher,
            generation: 0,
            search: None,
            shown_at: None,
            startup_notice: notice.is_some(),
            blur_checked: false,
            last_size: (0.0, 0.0),
        }
    }

    pub fn is_hint_state(&self) -> bool {
        self.results.is_empty() && self.query.trim().is_empty()
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        self.visible = true;
        self.fade = 0.0;
        self.shown_at = Some(Instant::now());
        self.focus_pending = true;
        self.last_size = (0.0, 0.0);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.centre(ctx);
        ctx.request_repaint();
    }

    pub fn hide(&mut self, ctx: &egui::Context) {
        self.visible = false;
        self.cancel_search();
        self.query.clear();
        self.results.clear();
        self.viewport.reset();
        self.pending_confirm = None;
        self.status = String::from("Ready");
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    pub fn toggle(&mut self, ctx: &egui::Context) {
        if self.visible {
            self.hide(ctx);
        } else {
            self.show(ctx);
        }
    }

    fn centre(&self, ctx: &egui::Context) {
        let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) else {
            return;
        };
        let width = self.config.appearance.width;
        let height = window_height(self.is_hint_state(), self.viewport.visible().len());
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
            (monitor.x - width) / 2.0,
            (monitor.y - height) / 3.0,
        )));
    }

    pub fn fit_window(&mut self, ctx: &egui::Context) {
        let width = self.config.appearance.width;
        let height = window_height(self.is_hint_state(), self.viewport.visible().len());
        if (width - self.last_size.0).abs() < 0.5 && (height - self.last_size.1).abs() < 0.5 {
            return;
        }
        self.last_size = (width, height);
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width, height)));
    }

    pub fn search_current(&mut self) {
        self.cancel_search();
        self.generation += 1;
        self.results.clear();
        self.viewport.reset();
        self.pending_confirm = None;

        let (body, prefixed, targets) = {
            let routed = self.router.route(&self.query);
            (
                routed.body.to_owned(),
                routed.prefixed,
                self.registry.select(&routed.targets),
            )
        };
        if body.is_empty() || targets.is_empty() {
            self.status = String::from("Ready");
            return;
        }
        let (rx, cancel) = engine::start(self.generation, body, prefixed, targets);
        self.search = Some((rx, cancel));
        self.status = String::from("Searching");
    }

    fn cancel_search(&mut self) {
        if let Some((_, cancel)) = self.search.take() {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    pub fn activate(&mut self, ctx: &egui::Context, index: usize) {
        let Some(item) = self.results.get(index).cloned() else {
            return;
        };
        if item.action == Action::Noop {
            return;
        }
        if item.confirm && self.pending_confirm != Some(index) {
            self.pending_confirm = Some(index);
            self.status = format!("Press Enter again to confirm {}", item.title);
            return;
        }
        self.pending_confirm = None;
        self.status = format!("Running {}", item.title);
        match launcher_platform::execute(item.action) {
            Ok(()) => {
                if self.config.general.close_on_launch {
                    self.hide(ctx);
                } else {
                    self.status = String::from("Done");
                }
            }
            Err(err) => self.status = format!("{err:#}"),
        }
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        let (down, up, tab, shift_tab, enter, escape, digit) = ctx.input_mut(|i| {
            let shift_tab = i.consume_key(Modifiers::SHIFT, Key::Tab);
            let tab = i.consume_key(Modifiers::NONE, Key::Tab);
            let digit = DIGITS.iter().position(|key| i.consume_key(Modifiers::CTRL, *key));
            let down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
            let up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
            let enter = i.consume_key(Modifiers::NONE, Key::Enter);
            let escape = i.consume_key(Modifiers::NONE, Key::Escape);
            (down, up, tab, shift_tab, enter, escape, digit)
        });

        if down || tab {
            self.viewport.step(1);
        }
        if up || shift_tab {
            self.viewport.step(-1);
        }
        if let Some(n) = digit {
            let visible = self.viewport.visible();
            if n < visible.len() {
                self.activate(ctx, visible.start + n);
            }
        }
        if enter && !self.results.is_empty() {
            let index = self.viewport.selected;
            self.activate(ctx, index);
        }
        if escape {
            if self.query.is_empty() {
                self.hide(ctx);
            } else {
                self.query.clear();
                self.search_current();
            }
        }
    }

    fn poll_hotkey(&mut self, ctx: &egui::Context) {
        let mut pressed = false;
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == HotKeyState::Pressed {
                pressed = true;
            }
        }
        if pressed {
            self.toggle(ctx);
        }
    }

    fn poll_config(&mut self, ctx: &egui::Context) {
        while let Ok(config) = self.config_rx.try_recv() {
            self.apply_config(ctx, config);
        }
    }

    fn poll_search(&mut self, ctx: &egui::Context) {
        let events: Vec<SearchEvent> = match &self.search {
            Some((rx, _)) => rx.try_iter().collect(),
            None => return,
        };
        for event in events {
            match event {
                SearchEvent::Items { generation, items } if generation == self.generation => {
                    for item in items {
                        self.add_result(ctx, item);
                    }
                    self.results.sort_by(|a, b| b.score.cmp(&a.score));
                    self.results.truncate(self.config.general.candidate_limit);
                    self.pending_confirm = None;
                    self.viewport.set_total(self.results.len());
                }
                SearchEvent::Done { generation } if generation == self.generation => {
                    self.status = if self.results.is_empty() {
                        String::from("No results")
                    } else {
                        format!("{} results", self.results.len())
                    };
                    self.search = None;
                }
                _ => {}
            }
        }
    }

    fn add_result(&mut self, ctx: &egui::Context, item: ResultItem) {
        if let Some(icon) = &item.icon {
            let key = icon_key(&item);
            if !self.icons.contains_key(&key) {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [icon.width as usize, icon.height as usize],
                    &icon.rgba,
                );
                let texture = ctx.load_texture(key.clone(), image, egui::TextureOptions::LINEAR);
                self.icons.insert(key, texture);
            }
        }
        self.results.push(item);
    }

    fn check_blur_dismiss(&mut self, ctx: &egui::Context) {
        let focused = ctx.input(|i| i.viewport().focused).unwrap_or(true);
        let grace_over = self.shown_at.map_or(true, |at| at.elapsed() >= BLUR_GRACE);
        if !focused && grace_over && self.config.general.close_on_blur {
            self.hide(ctx);
        }
    }

    fn reregister_hotkey(&mut self, summon: &str) -> Result<(), String> {
        let manager = self
            .hotkeys
            .as_ref()
            .ok_or_else(|| String::from("global hotkeys are unavailable"))?;
        let new: HotKey = summon.parse().map_err(|err| format!("{err}"))?;
        if let Some(old) = self.hotkey {
            manager.unregister(old).map_err(|err| err.to_string())?;
        }
        if let Err(err) = manager.register(new) {
            if let Some(old) = self.hotkey {
                let _ = manager.register(old);
            }
            return Err(err.to_string());
        }
        self.hotkey = Some(new);
        Ok(())
    }

    fn apply_config(&mut self, ctx: &egui::Context, config: Config) {
        let hotkey_status = if config.hotkey != self.config.hotkey {
            Some(match self.reregister_hotkey(&config.hotkey.summon) {
                Ok(()) => String::from("Hotkey updated"),
                Err(err) => format!("Hotkey not changed: {err}"),
            })
        } else {
            None
        };
        if config.priorities != self.config.priorities
            || config.general.auto_calculate != self.config.general.auto_calculate
        {
            self.registry = registry::build(&config);
        }
        self.viewport.set_capacity(config.general.max_visible_results);
        self.router = Router::new(&config.prefixes);
        self.theme = resolve_theme(&config);
        self.config = config;
        self.status = hotkey_status.unwrap_or_else(|| String::from("Config reloaded"));
        self.fit_window(ctx);
    }
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.startup_notice {
            self.startup_notice = false;
            self.show(ctx);
        }
        if !self.blur_checked {
            self.blur_checked = true;
            self.native_blur = launcher_platform::apply_backdrop(&*frame, self.config.appearance.blur);
            if !self.native_blur && self.config.appearance.blur != Blur::Off {
                self.status = String::from("Blur not available here, using a solid background");
            }
        }

        self.poll_hotkey(ctx);
        self.poll_config(ctx);
        self.poll_search(ctx);

        if self.visible {
            self.handle_keys(ctx);
            self.check_blur_dismiss(ctx);
            ui::show(self, ctx);
        }
        ctx.request_repaint_after(Duration::from_millis(50));
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

fn resolve_theme(config: &Config) -> ResolvedTheme {
    let system_dark = matches!(dark_light::detect(), Ok(dark_light::Mode::Dark));
    ResolvedTheme::resolve(config.theme.mode, &config.theme.custom, system_dark)
}

fn watch_config(tx: Sender<Config>) -> Option<notify::RecommendedWatcher> {
    let path = config_path();
    let dir = path.parent()?.to_path_buf();
    let target = path.clone();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            if event.paths.iter().any(|p| p == &target) {
                if let Ok(config) = Config::load(&target) {
                    let _ = tx.send(config);
                }
            }
        }
    })
    .ok()?;
    watcher.watch(&dir, RecursiveMode::NonRecursive).ok()?;
    Some(watcher)
}

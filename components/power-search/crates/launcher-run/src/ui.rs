use crate::app::{icon_key, Launcher, ROW_HEIGHT};
use eframe::egui::{self, Align2, Color32, FontId, Frame, Margin, Rect, RichText, Sense, Stroke, Vec2};
use launcher_settings::{config::Blur, Rgba};

pub fn colour(c: Rgba, alpha: f32) -> Color32 {
    let a = (f32::from(c.a) * alpha.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, a)
}

pub fn show(app: &mut Launcher, ctx: &egui::Context) {
    advance_fade(app, ctx);
    let theme = app.theme;
    let radius = app.config.appearance.corner_radius;
    let blur_fallback = app.config.appearance.blur != Blur::Off && !app.native_blur;
    let opacity = if blur_fallback {
        app.config.appearance.opacity.max(0.97)
    } else {
        app.config.appearance.opacity
    };
    let ease = ease_out(app.fade);
    let stroke = if app.config.appearance.border {
        Stroke::new(1.0, colour(theme.outline, 0.35 * ease))
    } else {
        Stroke::NONE
    };
    let frame = Frame::none()
        .fill(colour(theme.surface, opacity * ease))
        .rounding(app.config.appearance.corner_radius)
        .inner_margin(Margin::same(10.0))
        .stroke(stroke);

    egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
        ui.set_opacity(ease);
        ui.spacing_mut().item_spacing = Vec2::new(0.0, 6.0);
        search_box(app, ui, ctx);
        if app.is_hint_state() {
            hints(app, ui);
        } else {
            results(app, ui, ctx);
        }
        status(app, ui);
    });

    let _ = radius;
    app.fit_window(ctx);
}

fn advance_fade(app: &mut Launcher, ctx: &egui::Context) {
    let duration = app.config.animation.speed.duration().as_secs_f32();
    if !app.config.animation.fade_in || duration <= 0.0 {
        app.fade = 1.0;
        return;
    }
    if app.fade < 1.0 {
        let dt = ctx.input(|i| i.stable_dt);
        app.fade = (app.fade + dt / duration).min(1.0);
        ctx.request_repaint();
    }
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

fn search_box(app: &mut Launcher, ui: &mut egui::Ui, ctx: &egui::Context) {
    let theme = app.theme;
    ui.visuals_mut().extreme_bg_color = colour(theme.surface_container_high, 1.0);
    ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(1.0, colour(theme.outline, 1.0));
    ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::new(1.0, colour(theme.outline, 1.0));
    ui.visuals_mut().widgets.active.bg_stroke = Stroke::new(1.0, colour(theme.primary, 1.0));

    let response = egui::TextEdit::singleline(&mut app.query)
        .hint_text("Search, or type a prefix")
        .desired_width(f32::INFINITY)
        .font(FontId::proportional(18.0))
        .margin(Vec2::new(14.0, 10.0))
        .text_color(colour(theme.on_surface, 1.0))
        .show(ui)
        .response;

    if app.focus_pending {
        response.request_focus();
        app.focus_pending = false;
    }
    if response.changed() {
        app.search_current();
    }
    let _ = ctx;
}

fn hints(app: &Launcher, ui: &mut egui::Ui) {
    let theme = app.theme;
    ui.add_space(6.0);
    ui.label(
        RichText::new("Type to search, or start with a prefix")
            .size(13.0)
            .color(colour(theme.on_surface_variant, 1.0)),
    );
    ui.add_space(4.0);
    let prefixes: Vec<(String, String)> = app
        .config
        .prefixes
        .iter()
        .map(|(prefix, module)| (prefix.clone(), module.clone()))
        .collect();
    ui.horizontal_wrapped(|ui| {
        for (prefix, module) in prefixes {
            Frame::none()
                .fill(colour(theme.surface_container_high, 1.0))
                .rounding(10.0)
                .inner_margin(Margin::symmetric(10.0, 4.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("{}  {}", prefix.trim(), module))
                            .size(12.0)
                            .color(colour(theme.on_surface_variant, 1.0)),
                    );
                });
        }
    });
}

fn results(app: &mut Launcher, ui: &mut egui::Ui, ctx: &egui::Context) {
    let theme = app.theme;
    let radius = app.config.appearance.corner_radius;
    let pointer_moved = ui.input(|i| i.pointer.delta() != Vec2::ZERO);
    let mut clicked = None;

    for index in app.viewport.visible() {
        let item = &app.results[index];
        let title = item.title.clone();
        let subtitle = if app.pending_confirm == Some(index) {
            Some(String::from("Press Enter again to confirm"))
        } else {
            item.subtitle.clone()
        };
        let glyph = item.glyph;
        let texture = app.icons.get(&icon_key(item)).cloned();

        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_HEIGHT), Sense::click());
        if response.hovered() && pointer_moved {
            app.viewport.selected = index;
        }
        let selected = index == app.viewport.selected;

        let background = if selected {
            colour(theme.primary_container, 1.0)
        } else if response.hovered() {
            colour(theme.surface_container_high, 1.0)
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, radius * 0.6, background);

        let badge = Rect::from_min_size(rect.min + Vec2::new(8.0, 9.0), Vec2::splat(34.0));
        ui.painter()
            .rect_filled(badge, 10.0, colour(theme.surface_container_high, 1.0));
        match texture {
            Some(texture) => {
                let icon_rect = Rect::from_center_size(badge.center(), Vec2::splat(26.0));
                ui.put(
                    icon_rect,
                    egui::Image::new(&texture).fit_to_exact_size(Vec2::splat(26.0)),
                );
            }
            None => {
                ui.painter().text(
                    badge.center(),
                    Align2::CENTER_CENTER,
                    glyph.to_string(),
                    FontId::proportional(14.0),
                    colour(theme.on_primary_container, 1.0),
                );
            }
        }

        let text_colour = if selected {
            colour(theme.on_primary_container, 1.0)
        } else {
            colour(theme.on_surface, 1.0)
        };
        ui.painter().text(
            rect.min + Vec2::new(52.0, 9.0),
            Align2::LEFT_TOP,
            title,
            FontId::proportional(15.0),
            text_colour,
        );
        if let Some(subtitle) = subtitle {
            ui.painter().text(
                rect.min + Vec2::new(52.0, 29.0),
                Align2::LEFT_TOP,
                subtitle,
                FontId::proportional(12.0),
                colour(theme.on_surface_variant, 1.0),
            );
        }

        if response.clicked() {
            clicked = Some(index);
        }
    }

    if let Some(index) = clicked {
        app.activate(ctx, index);
    }
}

fn status(app: &Launcher, ui: &mut egui::Ui) {
    ui.add_space(2.0);
    ui.label(
        RichText::new(&app.status)
            .size(12.0)
            .color(colour(app.theme.on_surface_variant, 1.0)),
    );
}

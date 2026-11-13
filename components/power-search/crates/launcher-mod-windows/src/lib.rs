use launcher_platform as platform;
use launcher_core::{fuzzy, Action, Module, Query, ResultItem, Sink};

pub struct Windows;

impl Module for Windows {
    fn id(&self) -> &'static str {
        "windows"
    }

    fn query(&self, query: &Query, sink: &Sink) {
        let windows = match platform::list_windows() {
            Ok(windows) => windows,
            Err(err) => {
                if query.prefixed {
                    let mut notice = ResultItem::new(self.id(), "Window switching unavailable", Action::Noop);
                    notice.subtitle = Some(err.to_string());
                    sink.push(notice);
                }
                return;
            }
        };
        for window in windows {
            if query.cancelled() {
                return;
            }
            let haystack = format!("{} {}", window.title, window.app);
            let Some(score) = fuzzy::score(&query.body, &haystack) else {
                continue;
            };
            let subtitle = if window.app.is_empty() {
                String::from("Window")
            } else {
                window.app.clone()
            };
            let action = Action::Focus {
                handle: window.handle,
                app: window.app,
            };
            let mut item = ResultItem::new(self.id(), window.title, action);
            item.subtitle = Some(subtitle);
            item.score = i64::from(score);
            sink.push(item);
        }
    }
}

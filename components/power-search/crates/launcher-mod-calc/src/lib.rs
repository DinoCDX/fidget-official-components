use launcher_core::{Action, Module, Query, ResultItem, Sink};

pub struct Calculator {
    auto: bool,
}

impl Calculator {
    pub fn new(auto: bool) -> Self {
        Self { auto }
    }
}

impl Module for Calculator {
    fn id(&self) -> &'static str {
        "calc"
    }

    fn query(&self, query: &Query, sink: &Sink) {
        if !query.prefixed && !self.auto {
            return;
        }
        if !query.body.chars().any(|c| c.is_ascii_digit()) {
            return;
        }
        let Ok(value) = meval::eval_str(&query.body) else {
            return;
        };
        if !value.is_finite() || query.cancelled() {
            return;
        }
        let shown = format_number(value);
        let mut item = ResultItem::new(self.id(), shown.clone(), Action::Copy(shown));
        item.subtitle = Some(query.body.clone());
        item.score = 1000;
        sink.push(item);
    }
}

fn format_number(value: f64) -> String {
    let rounded = (value * 1e9).round() / 1e9;
    if rounded.fract() == 0.0 && rounded.abs() < 1e15 {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

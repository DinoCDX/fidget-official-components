use launcher_platform as platform;
use launcher_core::{fuzzy, Action, Module, Query, ResultItem, Sink};
use std::sync::OnceLock;

const LIMIT: usize = 50;
const ICON_LIMIT: usize = 12;

#[derive(Default)]
pub struct Apps {
    index: OnceLock<Vec<(String, Action)>>,
}

impl Module for Apps {
    fn id(&self) -> &'static str {
        "apps"
    }

    fn warm(&self) {
        self.index.get_or_init(platform::index_apps);
    }

    fn query(&self, query: &Query, sink: &Sink) {
        let index = self.index.get_or_init(platform::index_apps);
        let mut hits: Vec<(u32, &String, &Action)> = index
            .iter()
            .filter_map(|(name, action)| fuzzy::score(&query.body, name).map(|s| (s, name, action)))
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0));
        for (rank, (score, name, action)) in hits.into_iter().take(LIMIT).enumerate() {
            if query.cancelled() {
                return;
            }
            let mut item = ResultItem::new(self.id(), name.clone(), action.clone());
            item.subtitle = Some(String::from("Application"));
            item.score = i64::from(score);
            if rank < ICON_LIMIT {
                if let Action::Open(path) = action {
                    item.icon = launcher_platform::icon_for(path);
                }
            }
            sink.push(item);
        }
    }
}

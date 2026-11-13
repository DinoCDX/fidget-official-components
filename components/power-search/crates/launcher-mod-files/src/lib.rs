use launcher_core::{fuzzy, Action, Module, Query, ResultItem, Sink};
use std::{path::PathBuf, sync::OnceLock};
use walkdir::{DirEntry, WalkDir};

const LIMIT: usize = 30;
const ICON_LIMIT: usize = 12;
const INDEX_CAP: usize = 100_000;
const MAX_DEPTH: usize = 6;

#[derive(Default)]
pub struct Files {
    index: OnceLock<Vec<PathBuf>>,
}

impl Module for Files {
    fn id(&self) -> &'static str {
        "files"
    }

    fn warm(&self) {
        self.index.get_or_init(build_index);
    }

    fn query(&self, query: &Query, sink: &Sink) {
        let index = self.index.get_or_init(build_index);
        let mut hits: Vec<(u32, &PathBuf)> = index
            .iter()
            .filter_map(|path| {
                let name = path.file_name()?.to_string_lossy();
                fuzzy::score(&query.body, &name).map(|score| (score, path))
            })
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0));
        for (rank, (score, path)) in hits.into_iter().take(LIMIT).enumerate() {
            if query.cancelled() {
                return;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let mut item = ResultItem::new(self.id(), name, Action::Open(path.clone()));
            item.subtitle = path.parent().map(|p| p.display().to_string());
            item.score = i64::from(score);
            if rank < ICON_LIMIT {
                item.icon = launcher_platform::icon_for(path);
            }
            sink.push(item);
        }
    }
}

fn build_index() -> Vec<PathBuf> {
    let Some(dirs) = directories::UserDirs::new() else {
        return Vec::new();
    };
    WalkDir::new(dirs.home_dir())
        .max_depth(MAX_DEPTH)
        .into_iter()
        .filter_entry(|entry| !skip(entry))
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .take(INDEX_CAP)
        .map(DirEntry::into_path)
        .collect()
}

fn skip(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    name.starts_with('.')
        || matches!(
            name.as_ref(),
            "node_modules" | "target" | "AppData" | "Library" | "$RECYCLE.BIN"
        )
}

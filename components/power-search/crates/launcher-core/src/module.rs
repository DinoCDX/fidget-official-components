use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub type Cancel = Arc<AtomicBool>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemCommand {
    Lock,
    Sleep,
    Restart,
    Shutdown,
    EmptyRecycleBin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Launch { program: PathBuf, args: Vec<String> },
    Open(PathBuf),
    Focus { handle: u64, app: String },
    Copy(String),
    System(SystemCommand),
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconPixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultItem {
    pub module: &'static str,
    pub title: String,
    pub subtitle: Option<String>,
    pub glyph: char,
    pub score: i64,
    pub action: Action,
    pub confirm: bool,
    pub icon: Option<IconPixels>,
}

impl ResultItem {
    pub fn new(module: &'static str, title: impl Into<String>, action: Action) -> Self {
        let title = title.into();
        let glyph = title.chars().next().map_or('?', |c| c.to_ascii_uppercase());
        Self {
            module,
            title,
            subtitle: None,
            glyph,
            score: 0,
            action,
            confirm: false,
            icon: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SearchEvent {
    Items {
        generation: u64,
        items: Vec<ResultItem>,
    },
    Done {
        generation: u64,
    },
}

pub struct Query {
    pub body: String,
    pub prefixed: bool,
    pub cancel: Cancel,
}

impl Query {
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

pub struct Sink {
    priority: i64,
    cancel: Cancel,
    items: RefCell<Vec<ResultItem>>,
}

impl Sink {
    pub(crate) fn new(priority: i64, cancel: Cancel) -> Self {
        Self {
            priority,
            cancel,
            items: RefCell::new(Vec::new()),
        }
    }

    pub fn push(&self, mut item: ResultItem) {
        if self.cancel.load(Ordering::Relaxed) {
            return;
        }
        item.score += self.priority;
        self.items.borrow_mut().push(item);
    }

    pub(crate) fn drain(&self) -> Vec<ResultItem> {
        std::mem::take(&mut *self.items.borrow_mut())
    }
}

pub trait Module: Send + Sync {
    fn id(&self) -> &'static str;
    fn query(&self, query: &Query, sink: &Sink);
    fn warm(&self) {}
}

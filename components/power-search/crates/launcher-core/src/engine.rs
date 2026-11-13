use crate::{
    module::{Cancel, Module, Query, SearchEvent, Sink},
    router::Targets,
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::AtomicBool,
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
};

pub struct Registry {
    priorities: BTreeMap<String, i32>,
    modules: Vec<Arc<dyn Module>>,
}

impl Registry {
    pub fn new(priorities: BTreeMap<String, i32>) -> Self {
        Self {
            priorities,
            modules: Vec::new(),
        }
    }

    pub fn register(&mut self, module: Arc<dyn Module>) {
        self.modules.push(module);
    }

    pub fn select(&self, targets: &Targets) -> Vec<(Arc<dyn Module>, i64)> {
        self.modules
            .iter()
            .filter(|module| targets.matches(module.id()))
            .map(|module| {
                let priority = self.priorities.get(module.id()).copied().unwrap_or(0);
                (Arc::clone(module), i64::from(priority))
            })
            .collect()
    }

    pub fn warm(&self) {
        for module in &self.modules {
            let module = Arc::clone(module);
            thread::spawn(move || module.warm());
        }
    }
}

pub fn start(
    generation: u64,
    body: String,
    prefixed: bool,
    targets: Vec<(Arc<dyn Module>, i64)>,
) -> (Receiver<SearchEvent>, Cancel) {
    let (tx, rx) = mpsc::channel();
    let cancel: Cancel = Arc::new(AtomicBool::new(false));
    let mut workers = Vec::with_capacity(targets.len());

    for (module, priority) in targets {
        let tx = tx.clone();
        let query = Query {
            body: body.clone(),
            prefixed,
            cancel: cancel.clone(),
        };
        let sink = Sink::new(priority, cancel.clone());
        workers.push(thread::spawn(move || {
            module.query(&query, &sink);
            let _ = tx.send(SearchEvent::Items {
                generation,
                items: sink.drain(),
            });
        }));
    }

    thread::spawn(move || {
        for worker in workers {
            let _ = worker.join();
        }
        let _ = tx.send(SearchEvent::Done { generation });
    });

    (rx, cancel)
}

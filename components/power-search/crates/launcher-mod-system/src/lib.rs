use launcher_core::{fuzzy, Action, Module, Query, ResultItem, Sink, SystemCommand};

pub struct Commands;

const COMMANDS: &[(&str, &str, SystemCommand, bool)] = &[
    ("Lock", "Lock the workstation", SystemCommand::Lock, false),
    ("Sleep", "Suspend the device", SystemCommand::Sleep, false),
    ("Restart", "Restart the computer", SystemCommand::Restart, true),
    ("Shutdown", "Shut down the computer", SystemCommand::Shutdown, true),
    (
        "Empty recycle bin",
        "Permanently remove deleted files",
        SystemCommand::EmptyRecycleBin,
        true,
    ),
];

impl Module for Commands {
    fn id(&self) -> &'static str {
        "system"
    }

    fn query(&self, query: &Query, sink: &Sink) {
        for (name, description, command, confirm) in COMMANDS {
            if query.cancelled() {
                return;
            }
            let Some(score) = fuzzy::score(&query.body, name) else {
                continue;
            };
            let mut item = ResultItem::new(self.id(), *name, Action::System(*command));
            item.subtitle = Some(if *confirm {
                format!("{description} (requires confirmation)")
            } else {
                description.to_string()
            });
            item.confirm = *confirm;
            item.score = i64::from(score);
            sink.push(item);
        }
    }
}

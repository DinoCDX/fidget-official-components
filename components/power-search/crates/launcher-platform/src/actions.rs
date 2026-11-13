use crate::{focus, open_path, system};
use anyhow::Result;
use launcher_core::Action;

pub fn execute(action: Action) -> Result<()> {
    match action {
        Action::Launch { program, args } => {
            std::process::Command::new(program).args(args).spawn()?;
            Ok(())
        }
        Action::Open(path) => open_path(&path),
        Action::Focus { handle, app } => focus(handle, &app),
        Action::Copy(text) => {
            arboard::Clipboard::new()?.set_text(text)?;
            Ok(())
        }
        Action::System(command) => system(command),
        Action::Noop => Ok(()),
    }
}

use std::io::{Stdout, Write};

use crossterm::{
    QueueableCommand, cursor,
    style::{Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use tracing::{debug, info};

use crate::{
    error::{Result, UiError},
    registry::Entry,
    repl::{Router, parser::Parsed},
    runtime::ReplSettings,
    shell::ShellSpec,
};

pub fn render_prompt_line(
    router: &Router,
    out: &mut Stdout,
    buf: &str,
    settings: &ReplSettings,
) -> Result<()> {
    debug!(len = buf.len(), "render_prompt_line start");

    out.queue(cursor::MoveToColumn(0))
        .map_err(UiError::IoWrite)?;
    out.queue(Clear(ClearType::CurrentLine))
        .map_err(UiError::IoWrite)?;

    out.queue(Print("> ")).map_err(UiError::IoWrite)?;

    if let Some(colon_idx) = buf.find(':') {
        let prefix = &buf[..colon_idx];
        let rest = &buf[colon_idx + 1..];

        let parsed = router.parse_preview(buf);

        let color = match parsed {
            Parsed::Entry {
                ref name,
                ref entry,
                ..
            } if name == prefix => match entry {
                Entry::Builtin => settings.color_builtin,
                Entry::Shell(ShellSpec::Local { .. }) => settings.color_local,
                Entry::Shell(ShellSpec::Remote { .. }) => settings.color_remote,
            },
            _ => settings.color_unknown,
        };

        out.queue(SetForegroundColor(color))
            .map_err(UiError::IoWrite)?;
        out.queue(Print(prefix)).map_err(UiError::IoWrite)?;
        out.queue(Print(":")).map_err(UiError::IoWrite)?;
        out.queue(ResetColor).map_err(UiError::IoWrite)?;
        out.queue(Print(rest)).map_err(UiError::IoWrite)?;
    } else {
        out.queue(Print(buf)).map_err(UiError::IoWrite)?;
    }

    out.flush().map_err(UiError::IoWrite)?;

    #[cfg(feature = "ui-verbose")]
    info!("render_prompt_line ok");

    Ok(())
}

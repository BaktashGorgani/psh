use std::io::{Write, stdout};

use nu_ansi_term::{Color, Style};
use reedline::{Reedline, Signal};
use tracing::{debug, error, info, warn};

use crate::{
    error::{Result, UiError},
    registry,
    repl::Router,
    runtime::ReplSettings,
    shell::ShellSpec,
    ui::PshPrompt,
};

fn color_for_entry(entry: &registry::Entry, settings: &ReplSettings) -> Color {
    match entry {
        registry::Entry::Builtin => settings.color_builtin,
        registry::Entry::Shell(ShellSpec::Local { .. }) => settings.color_local,
        registry::Entry::Shell(ShellSpec::Remote { .. }) => settings.color_remote,
    }
}

fn print_menu(
    entries: &[(String, registry::Entry, bool)],
    settings: &ReplSettings,
    current: Option<&str>,
) -> Result<()> {
    debug!("prefix_menu_print start");
    let mut out = stdout();
    out.write_all(b"\r\n").map_err(UiError::IoWrite)?;
    out.write_all(b"Select a prefix by number or name. Empty line cancels.\r\n")
        .map_err(UiError::IoWrite)?;
    for (i, (name, entry, running)) in entries.iter().enumerate() {
        let color = color_for_entry(entry, settings);
        let styled = Style::new().fg(color).paint(name);
        let marker = if Some(name.as_str()) == current {
            ">"
        } else {
            " "
        };
        let run = if *running { "*" } else { " " };
        let line = format!("{:>2}. {} {} {}\r\n", i + 1, marker, run, styled);
        out.write_all(line.as_bytes()).map_err(UiError::IoWrite)?;
    }
    out.flush().map_err(UiError::IoWrite)?;
    info!("prefix_menu_print ok");
    Ok(())
}

pub async fn choose_prefix(
    router: &Router,
    settings: &ReplSettings,
    current: Option<&str>,
) -> Result<Option<String>> {
    debug!("prefix_menu start");
    let entries = router.list_entries_with_status().await;
    if entries.is_empty() {
        warn!("prefix_menu empty");
        return Ok(None);
    }

    print_menu(&entries, settings, current)?;

    let mut rl = Reedline::create();
    let mut menu_prompt = PshPrompt::new(settings);
    menu_prompt.set_registry(router.get_registry_clone());

    loop {
        let sig = rl.read_line(&menu_prompt);
        match sig {
            Ok(Signal::Success(line)) => {
                let input = line.trim();
                if input.is_empty() {
                    info!("prefix_menu canceled");
                    return Ok(None);
                }

                if let Ok(idx) = input.parse::<usize>() {
                    if idx >= 1 && idx <= entries.len() {
                        let chosen = entries[idx - 1].0.clone();
                        info!(chosen = %chosen, "prefix_menu invalid index");
                        return Ok(Some(chosen));
                    } else {
                        warn!(input = input, "prefix_menu invalid index");
                    }
                } else if let Some((name, _, _)) =
                    entries.iter().find(|(n, _, _)| n == input)
                {
                    let chosen = name.clone();
                    info!(chosen = %chosen, "prefix_menu ok");
                    return Ok(Some(chosen));
                } else {
                    warn!(input = input, "prefix_menu unkown name");
                }

                let mut out = stdout();
                out.write_all(b"\r\n").map_err(UiError::IoWrite)?;
                out.flush().map_err(UiError::IoWrite)?;
                print_menu(&entries, settings, current)?;
            }
            Ok(Signal::CtrlC) => {
                info!("prefix_menu ctr_c_cancel");
                return Ok(None);
            }
            Ok(Signal::CtrlD) => {
                info!("prefix_menu ctr_d_cancel");
                return Ok(None);
            }
            Err(e) => {
                error!(?e, "prefix_menu read_line failed");
                return Err(UiError::IoRead(e).into());
            }
        }
    }
}

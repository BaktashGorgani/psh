use std::io::{Stdout, Write};

use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event, KeyCode, KeyEvent},
    style::{Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, ScrollUp},
};
use tracing::{debug, error, info, warn};

use crate::{
    error::{Result, UiError},
    registry,
    repl::Router,
    runtime::ReplSettings,
    shell::ShellSpec,
};

pub async fn choose_prefix(
    router: &Router,
    out: &mut Stdout,
    settings: &ReplSettings,
    current: Option<&str>,
) -> Result<Option<String>> {
    debug!("prefix_menu start");
    let entries = router.list_entries_with_status().await;
    if entries.is_empty() {
        warn!("prefix_menu empty");
        return Ok(None);
    }

    let mut selected = current
        .and_then(|c| entries.iter().position(|(n, _, _)| n == c))
        .unwrap_or(0usize);

    let (_, mut base_row) = cursor::position().map_err(UiError::IoWrite)?;
    let (_, term_rows) = terminal::size().map_err(UiError::IoWrite)?;
    let mut available = term_rows.saturating_sub(base_row + 2);
    let mut visible_count = entries.len().min(usize::from(available.max(1)));
    let mut scroll = 0usize;

    let needed = 1u16.saturating_add(u16::try_from(visible_count).unwrap_or(u16::MAX));
    let y_last = base_row.saturating_add(needed);

    if y_last >= term_rows {
        let overflow = y_last - (term_rows - 1);
        out.queue(ScrollUp(overflow)).map_err(UiError::IoWrite)?;
        base_row = base_row.saturating_sub(overflow);
        available = term_rows.saturating_sub(base_row + 2);
        let new_visible = entries.len().min(usize::from(available.max(1)));
        if new_visible != visible_count {
            visible_count = new_visible
        }
        info!(overflow, "prefix_menu scrolled");
    }

    let draw = |out: &mut Stdout, scroll: usize, selected: usize| -> Result<()> {
        for i in 0..visible_count {
            let idx = scroll + i;
            let y = base_row + 1 + u16::try_from(i).unwrap_or(0);
            out.queue(cursor::MoveTo(0, y)).map_err(UiError::IoWrite)?;
            out.queue(Clear(ClearType::CurrentLine))
                .map_err(UiError::IoWrite)?;

            if idx >= entries.len() {
                continue;
            }

            let (ref name, ref entry, running) = entries[idx];
            let color = match entry {
                registry::Entry::Builtin => settings.color_builtin,
                registry::Entry::Shell(ShellSpec::Local { .. }) => settings.color_local,
                registry::Entry::Shell(ShellSpec::Remote { .. }) => {
                    settings.color_remote
                }
            };

            let sel = if idx == selected { ">" } else { " " };
            let run = if running { "*" } else { " " };
            let label = format!("{sel} {run} {name}");

            out.queue(SetForegroundColor(color))
                .map_err(UiError::IoWrite)?;
            out.queue(Print(label)).map_err(UiError::IoWrite)?;
            out.queue(ResetColor).map_err(UiError::IoWrite)?;
        }

        let hint_row = base_row + 1 + u16::try_from(visible_count).unwrap_or(0);
        out.queue(cursor::MoveTo(0, hint_row))
            .map_err(UiError::IoWrite)?;
        out.queue(Clear(ClearType::CurrentLine))
            .map_err(UiError::IoWrite)?;
        out.queue(Print("↑/↓ or j/k to move, Enter select, Esc cancel"))
            .map_err(UiError::IoWrite)?;
        out.flush().map_err(UiError::IoWrite)?;
        info!("prefix_menu draw ok");
        Ok(())
    };

    draw(out, scroll, selected)?;

    let chosen = loop {
        match event::read() {
            Ok(Event::Key(KeyEvent { code, .. })) => match code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    if selected > 0 {
                        selected -= 1;
                        if selected < scroll {
                            scroll = selected;
                        }
                        debug!(selected, scroll, "prefix_menu move up");
                        draw(out, scroll, selected)?;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    if selected + 1 < entries.len() {
                        selected += 1;
                        if selected >= scroll + visible_count {
                            scroll = selected + 1 - visible_count;
                        }
                        debug!(selected, scroll, "prefix_menu move down");
                        draw(out, scroll, selected)?;
                    }
                }
                KeyCode::PageUp => {
                    let step = visible_count.max(1);
                    let new_sel = selected.saturating_sub(step);
                    if new_sel != selected {
                        selected = new_sel;
                        if selected < scroll {
                            scroll = selected;
                        }
                        debug!(selected, scroll, "prefix_menu page up");
                        draw(out, scroll, selected)?;
                    }
                }
                KeyCode::PageDown => {
                    let step = visible_count.max(1);
                    let new_sel =
                        (selected + step).min(entries.len().saturating_sub(1));
                    if new_sel != selected {
                        selected = new_sel;
                        if selected >= scroll + visible_count {
                            scroll = selected + 1 - visible_count;
                        }
                        debug!(selected, scroll, "prefix_menu page down");
                        draw(out, scroll, selected)?;
                    }
                }
                KeyCode::Home => {
                    if selected != 0 {
                        selected = 0;
                        scroll = 0;
                        debug!(selected, scroll, "prefix_menu home");
                        draw(out, scroll, selected)?;
                    }
                }
                KeyCode::End => {
                    let last = entries.len().saturating_sub(1);
                    if selected != last {
                        selected = last;
                        scroll =
                            selected.saturating_sub(visible_count.saturating_sub(1));
                        debug!(selected, scroll, "prefix_menu end");
                        draw(out, scroll, selected)?;
                    }
                }
                KeyCode::Enter => break Some(entries[selected].0.clone()),
                KeyCode::Esc => break None,
                KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                    let offset = (c as u8 - b'1') as usize;
                    if offset < visible_count && scroll + offset < entries.len() {
                        break Some(entries[scroll + offset].0.clone());
                    }
                }
                _ => {}
            },
            Ok(_) => {}
            Err(e) => {
                error!(?e, "prefix_menu read failed");
                break None;
            }
        }
    };

    for i in 0..=visible_count {
        let y = base_row + 1 + u16::try_from(i).unwrap_or(0);
        out.queue(cursor::MoveTo(0, y)).map_err(UiError::IoWrite)?;
        out.queue(Clear(ClearType::CurrentLine))
            .map_err(UiError::IoWrite)?;
    }
    out.queue(cursor::MoveTo(0, base_row))
        .map_err(UiError::IoWrite)?;
    out.flush().map_err(UiError::IoWrite)?;

    match chosen {
        Some(ref name) => info!(chosen = %name, "prefix_menu ok"),
        None => info!("prefix_menu canceled"),
    }
    Ok(chosen)
}

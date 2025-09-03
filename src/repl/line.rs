use std::io::{Write, stdout};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self},
};
use tracing::{debug, error, info, warn};

use crate::{
    PshError,
    error::{BuiltinError, Result, UiError},
    registry,
    repl::{Router, parser::Parsed, prompt::render_prompt_line},
    runtime::ReplSettings,
    shell::Shell,
};

const CTRL_C_LITERAL: u8 = 0x03;
const CRLF: &[u8] = b"\r\n";

pub async fn run(router: &mut Router, settings: &ReplSettings) -> Result<()> {
    debug!("repl_line_run start");
    println!("Type lines like:");
    println!(
        " bash: <cmd> | zsh: <cmd> | local: list | local: add mysh zsh | remote: add r1 user@host | remote: connect r1 | admin: default get | admin: default set bash | quit"
    );

    terminal::enable_raw_mode().map_err(|e| UiError::RawModeEnable(e.into()))?;
    info!("raw mode enabled");

    let mut out = stdout();
    let mut buf = String::new();

    render_prompt_line(router, &mut out, &buf, settings)?;

    loop {
        match event::read() {
            Ok(Event::Key(KeyEvent {
                code, modifiers, ..
            })) => {
                if matches_menu_key(code, modifiers, settings.menu_key) {
                    info!("prefix menu requested");
                    continue;
                }

                match code {
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        info!("ctrl-c requested");
                        let target_shell = match router.parse_preview(&buf) {
                            Parsed::Entry { name, entry, .. } => match entry {
                                registry::Entry::Shell(_) => Some(name),
                                _ => router.get_default_shell(),
                            },
                            Parsed::Default { .. } => router.get_default_shell(),
                        };

                        if let Some(name) = target_shell {
                            let res = match router
                                .ensure_shell_session_by_name(&name)
                                .await
                            {
                                Ok(s) => s.send_bytes(vec![CTRL_C_LITERAL]).await,
                                Err(e) => Err(e),
                            };
                            match res {
                                Ok(()) => info!(shell = %name, "ctrl_c_forwarded"),
                                Err(e) => {
                                    warn!(?e, shell = %name, "ctrl_c_forward_failed")
                                }
                            }
                        } else {
                            debug!("ctrl_c no_target_shell");
                        }

                        if let Err(e) = (|| -> Result<()> {
                            out.write_all(CRLF).map_err(UiError::IoWrite)?;
                            Ok(())
                        })() {
                            error!(?e, "ctrl_c newline write failed");
                        }

                        buf.clear();
                        if let Err(e) =
                            render_prompt_line(router, &mut out, &buf, settings)
                        {
                            error!(?e, "render_prompt_line failed after ctrl-c");
                        }
                    }
                    KeyCode::Char(ch) => {
                        buf.push(ch);
                        if let Err(e) =
                            render_prompt_line(router, &mut out, &buf, settings)
                        {
                            error!(?e, "render_line failed")
                        }
                    }
                    KeyCode::Backspace => {
                        if !buf.is_empty() {
                            buf.pop();
                            if let Err(e) =
                                render_prompt_line(router, &mut out, &buf, settings)
                            {
                                error!(?e, "render_line failed");
                            }
                        } else {
                            debug!("backspace on empty buffer");
                        }
                    }
                    KeyCode::Enter => {
                        out.write_all(CRLF).map_err(UiError::IoWrite)?;

                        let line = buf.trim().to_string();
                        if !line.is_empty() {
                            match router.exec(&line).await {
                                Ok(_) => {}
                                Err(PshError::Builtin(BuiltinError::ExitRequested)) => {
                                    info!("quit via builtin");
                                    break;
                                }
                                Err(e) => {
                                    error!(?e, "router exec failed");
                                    out.write_all(b"\r").map_err(UiError::IoWrite)?;
                                    out.write_all(format!("error: {e}").as_bytes())
                                        .map_err(UiError::IoWrite)?;
                                    out.write_all(CRLF).map_err(UiError::IoWrite)?;
                                    out.flush().map_err(UiError::IoWrite)?;
                                    writeln!(out, "error: {e}")
                                        .map_err(UiError::IoWrite)?;
                                }
                            }
                        }
                        buf.clear();
                        if let Err(e) =
                            render_prompt_line(router, &mut out, &buf, settings)
                        {
                            error!(?e, "render_line failed after enter");
                        }
                    }
                    KeyCode::Esc => {
                        debug!("escape requested");
                    }
                    _ => debug!("ignored key"),
                }
            }
            Ok(Event::Resize(_, _)) => {
                if let Err(e) = render_prompt_line(router, &mut out, &buf, settings) {
                    error!(?e, "render_line failed on resize");
                }
            }
            Ok(_) => debug!("ignored non-key event"),
            Err(e) => {
                error!(?e, "event read failed");
                break;
            }
        }
    }

    match terminal::disable_raw_mode() {
        Ok(_) => info!("raw mode disabled"),
        Err(e) => error!(?e, "raw mode disable failed"),
    }

    Ok(())
}

fn normalize_event_key(code: KeyCode, mods: KeyModifiers) -> (KeyCode, KeyModifiers) {
    match code {
        KeyCode::Char(c) if c.is_ascii_uppercase() => (
            KeyCode::Char(c.to_ascii_lowercase()),
            mods | KeyModifiers::SHIFT,
        ),
        other => (other, mods),
    }
}

fn matches_menu_key(
    code: KeyCode,
    mods: KeyModifiers,
    want: (KeyCode, KeyModifiers),
) -> bool {
    let (c, m) = normalize_event_key(code, mods);
    c == want.0 && m.contains(want.1)
}

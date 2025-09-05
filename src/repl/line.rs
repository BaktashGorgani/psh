use std::io::{Write, stdout};

use reedline::Signal;
use tracing::{debug, error, info, warn};

use crate::{
    PshError,
    error::{BuiltinError, Result, UiError},
    repl::{Router, parser::Parsed},
    runtime::ReplSettings,
    shell::Shell,
    ui::{
        PshPrompt,
        editor::keymap::{MENU_SENTINEL, make_reedline},
        prefix_menu::choose_prefix,
    },
};

const CTRL_C_LITERAL: u8 = 0x03;

pub async fn run(router: &mut Router, settings: &ReplSettings) -> Result<()> {
    debug!("repl_line_run start");
    println!("Type lines like:");
    println!(
        " bash: <cmd> | zsh: <cmd> | local: list | local: add mysh zsh | remote: add r1 user@host | remote: connect r1 | admin: default get | admin: default set bash | quit"
    );

    let mut rl = make_reedline(settings);
    info!("reedline create ok");

    let mut prompt = PshPrompt::new(settings);
    prompt.set_registry(router.get_registry_clone());
    prompt.set_mode_state(router.mode_state());

    loop {
        match rl.read_line(&prompt) {
            Ok(sig) => match sig {
                Signal::Success(line) => {
                    debug!(len = line.len(), "read_line success");
                    let line = line.trim().to_string();

                    if line == MENU_SENTINEL {
                        debug!("menu sentinel detected");
                        let current = router.get_current_mode();
                        match choose_prefix(router, settings, current.as_deref())
                            .await?
                        {
                            Some(new_name) => {
                                router.set_current_mode(&new_name);
                                let mut out = stdout();
                                out.write_all(b"\r\n").map_err(UiError::IoWrite)?;
                                out.flush().map_err(UiError::IoWrite)?;
                                info!(name = %new_name, "menu selection set_current_mode ok");
                            }
                            None => info!("menu canceled"),
                        }
                        continue;
                    }

                    if line.is_empty() {
                        continue;
                    }

                    match router.parse_preview(&line) {
                        Parsed::Entry { name, .. } => {
                            router.set_current_mode(&name);
                            info!(name = %name, "explicit prefix set_current_mode ok");
                        }
                        Parsed::Default { .. } => {}
                    }

                    match router.exec(&line).await {
                        Ok(()) => info!("router exec ok"),
                        Err(PshError::Builtin(BuiltinError::ExitRequested)) => {
                            info!("quit via builtin");
                            break;
                        }
                        Err(e) => {
                            error!(?e, "router exec failed");
                            let mut out = stdout();
                            out.write_all(b"\r").map_err(UiError::IoWrite)?;
                            out.write_all(format!("error: {e}").as_bytes())
                                .map_err(UiError::IoWrite)?;
                            out.write_all(b"\r\n").map_err(UiError::IoWrite)?;
                            out.flush().map_err(UiError::IoWrite)?;
                        }
                    }
                }
                Signal::CtrlC => {
                    info!("ctrl-c requested");
                    if let Some(name) = router.get_current_mode() {
                        let res = match router.ensure_shell_session_by_name(&name).await
                        {
                            Ok(s) => s.send_bytes(vec![CTRL_C_LITERAL]).await,
                            Err(e) => Err(e),
                        };
                        match res {
                            Ok(()) => info!(shell = %name, "ctrl_c forwarded"),
                            Err(e) => warn!(?e,shell = %name, "ctrl_c forward_failed"),
                        }
                    } else {
                        debug!("ctrl_c no_default_shell");
                    }

                    let mut out = stdout();
                    out.write_all(b"\r\n").map_err(UiError::IoWrite)?;
                    out.flush().map_err(UiError::IoWrite)?;
                }
                Signal::CtrlD => {
                    info!("ctrl-d (eof) requester");
                    break;
                }
            },
            Err(e) => {
                error!(?e, "read_line error");
                return Err(UiError::IoRead(e).into());
            }
        }
    }

    info!("repl_line_run done");
    Ok(())
}

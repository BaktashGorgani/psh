use std::io::{Write, stdout};

use reedline::{Reedline, Signal};
use tracing::{debug, error, info, warn};

use crate::{
    PshError,
    error::{BuiltinError, Result, UiError},
    repl::Router,
    runtime::ReplSettings,
    shell::Shell,
    ui::PshPrompt,
};

const CTRL_C_LITERAL: u8 = 0x03;

pub async fn run(router: &mut Router, settings: &ReplSettings) -> Result<()> {
    debug!("repl_line_run start");
    println!("Type lines like:");
    println!(
        " bash: <cmd> | zsh: <cmd> | local: list | local: add mysh zsh | remote: add r1 user@host | remote: connect r1 | admin: default get | admin: default set bash | quit"
    );

    let mut rl = Reedline::create();
    info!("reedline create ok");

    let mut prompt = PshPrompt::new(router.get_default_shell(), settings);
    prompt.set_registry(router.get_registry_clone());

    loop {
        match rl.read_line(&prompt) {
            Ok(sig) => match sig {
                Signal::Success(line) => {
                    debug!(len = line.len(), "read_line success");
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
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
                    if let Some(name) = router.get_default_shell() {
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

        prompt.set_locked_prefix(router.get_default_shell());
    }

    info!("repl_line_run done");
    Ok(())
}

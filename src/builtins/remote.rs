use tracing::{debug, info, warn};

use crate::{
    builtins::{BuiltinContext, format_shell_line},
    error::{BuiltinError, Result},
    registry,
    shell::{ShellSpec, spec::RemoteBackend},
    ui::ui_println,
};

const DEFAULT_SSH_PORT: u16 = 22;
const DEFAULT_TELNET_PORT: u16 = 23;

pub async fn handle(ctx: &mut dyn BuiltinContext, args: &str) -> Result<()> {
    debug!(args = args, "builtin_remote_handle start");
    let parts: Vec<&str> = args.split_whitespace().collect();
    match parts.as_slice() {
        [] | ["list"] => {
            let mut printed = false;
            for (name, entry, running) in ctx.list_entries_with_status().await {
                if let registry::Entry::Shell(spec) = entry
                    && let ShellSpec::Remote { .. } = spec
                {
                    if !printed {
                        ui_println("Remote shell list:")?;
                        printed = true;
                    }
                    let line = format_shell_line(&name, &spec, running);
                    ui_println(&line)?;
                }
            }
            if !printed {
                ui_println("No remote shells registered")?;
            }
            info!("remote_list ok");
        }
        ["add", name, "ssh", dest] => {
            ctx.add_and_start_shell(
                name.to_string(),
                ShellSpec::Remote {
                    host: dest.to_string(),
                    backend: RemoteBackend::Ssh {
                        port: DEFAULT_SSH_PORT,
                        extra_args: vec![],
                    },
                },
            )
            .await?;
            info!(remote = *name, "remote_add ok");
        }
        ["add", name, "ssh", dest, rest @ ..] => {
            let mut port = DEFAULT_SSH_PORT;
            let mut extra_args: Vec<String> = Vec::new();
            if let Some(first) = rest.first() {
                extra_args = match first.parse::<u16>() {
                    Ok(p) => {
                        port = p;
                        rest.iter().skip(1).map(|s| s.to_string()).collect()
                    }
                    Err(_) => rest.iter().map(|s| s.to_string()).collect(),
                }
            }
            ctx.add_and_start_shell(
                name.to_string(),
                ShellSpec::Remote {
                    host: dest.to_string(),
                    backend: RemoteBackend::Ssh { port, extra_args },
                },
            )
            .await?;
            info!(remote = *name, "remote_add ok");
        }
        ["add", name, "telnet", dest] => {
            ctx.add_and_start_shell(
                name.to_string(),
                ShellSpec::Remote {
                    host: dest.to_string(),
                    backend: RemoteBackend::Telnet {
                        port: DEFAULT_TELNET_PORT,
                        extra_args: vec![],
                    },
                },
            )
            .await?;
            info!(remote = *name, "remote_add ok");
        }
        ["add", name, "telnet", dest, rest @ ..] => {
            let mut port = DEFAULT_TELNET_PORT;
            let mut extra_args: Vec<String> = Vec::new();
            if let Some(first) = rest.first() {
                extra_args = match first.parse::<u16>() {
                    Ok(p) => {
                        port = p;
                        rest.iter().skip(1).map(|s| s.to_string()).collect()
                    }
                    Err(_) => rest.iter().map(|s| s.to_string()).collect(),
                }
            }
            ctx.add_and_start_shell(
                name.to_string(),
                ShellSpec::Remote {
                    host: dest.to_string(),
                    backend: RemoteBackend::Telnet { port, extra_args },
                },
            )
            .await?;
            info!(remote = *name, "remote_add ok");
        }
        ["remove", name] => {
            let _ = ctx.stop_shell_session(name).await;
            ctx.unregister_entry(name);
            info!(remote = *name, "remote_remove ok")
        }
        ["connect", name] => {
            ctx.ensure_shell_session_by_name(name).await?;
            info!(remote = %name, "remote_connect ok")
        }
        ["disconnect", name] => {
            ctx.stop_shell_session(name).await?;
            info!(remote = %name, "remote_disconnect ok")
        }
        _ => {
            warn!(args = args, "remote_unrecognized");
            return Err(BuiltinError::RemoteUnrecognized {
                args: args.to_string(),
            }
            .into());
        }
    }
    info!("builtin_remote_handle ok");
    Ok(())
}

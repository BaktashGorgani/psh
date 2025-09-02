use tracing::{debug, info, warn};

use crate::{
    builtins::BuiltinContext,
    error::{BuiltinError, Result},
    registry::Entry,
    shell::ShellSpec,
    ui::ui_println,
};

pub async fn handle(ctx: &mut dyn BuiltinContext, args: &str) -> Result<()> {
    debug!(args = args, "builtin_remote_handle start");
    let parts: Vec<&str> = args.split_whitespace().collect();
    match parts.as_slice() {
        [] | ["list"] => {
            let mut any = false;
            for (name, entry, running) in ctx.list_entries_with_status().await {
                if let Entry::Shell(ShellSpec::Remote { target, port, .. }) = entry {
                    if !any {
                        ui_println("Remote shell list:")?;
                        any = true;
                    }
                    let status = if running {
                        "[connected]"
                    } else {
                        "[disconnected]"
                    };
                    let mut line = format!("  {}: {}", name, target);
                    match port {
                        Some(p) => line.push_str(&format!(":{p}")),
                        None => line.push_str(":22"),
                    }
                    line.push_str(&format!(" {status}"));
                    ui_println(&line)?;
                }
            }
            if !any {
                ui_println("No remote shells registered")?;
            }
            info!("remote_list ok");
        }
        ["add", name, dest] => {
            ctx.add_and_start_shell(
                (*name).to_string(),
                ShellSpec::Remote {
                    target: dest.to_string(),
                    port: None,
                    extra_args: vec![],
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

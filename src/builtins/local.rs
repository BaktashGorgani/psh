use tracing::{debug, info, warn};

use crate::{
    builtins::BuiltinContext,
    error::{BuiltinError, Result},
    registry::Entry,
    shell::ShellSpec,
    ui::ui_println,
};

pub async fn handle(ctx: &mut dyn BuiltinContext, args: &str) -> Result<()> {
    debug!(args = args, "builtin_local_handle start");
    let parts: Vec<&str> = args.split_whitespace().collect();
    match parts.as_slice() {
        [] | ["list"] => {
            let mut any = false;
            for (name, entry, running) in ctx.list_entries_with_status().await {
                if let Entry::Shell(ShellSpec::Local { program }) = entry {
                    if !any {
                        ui_println("Local shell list:")?;
                        any = true;
                    }
                    let status = if running { "[running]" } else { "[stopped]" };
                    ui_println(&format!("  {name}: {program} {status}"))?;
                }
            }
            if !any {
                ui_println("No local shells registered")?;
            }
            info!("local_list ok");
        }
        ["add", name, program] => {
            ctx.add_and_start_shell(
                (*name).to_string(),
                ShellSpec::Local {
                    program: (*program).to_string(),
                },
            )
            .await?;
            info!(name = *name, program = *program, "local_add_and_start ok");
        }
        ["remove", name] => {
            let _ = ctx.stop_shell_session(name).await;
            ctx.unregister_entry(name);
            info!(name = *name, "local_remove ok");
        }
        ["start", name] => {
            ctx.ensure_shell_session_by_name(name).await?;
            info!(name = *name, "local_start ok");
        }
        ["stop", name] => {
            ctx.stop_shell_session(name).await?;
            info!(name = *name, "local_stop ok");
        }
        _ => {
            warn!(args = args, "local_unrecognized");
            return Err(BuiltinError::LocalUnrecognized {
                args: args.to_string(),
            }
            .into());
        }
    }
    info!("builtin_local_handle ok");
    Ok(())
}

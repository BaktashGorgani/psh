use tracing::{debug, info, warn};

use crate::{
    builtins::BuiltinContext,
    error::{BuiltinError, Result},
    ui::ui_println,
};

pub async fn handle(ctx: &mut dyn BuiltinContext, args: &str) -> Result<()> {
    debug!(args = args, "builtin_admin_handle start");

    let parts: Vec<&str> = args.split_whitespace().collect();

    match parts.as_slice() {
        ["sessions"] => {
            ui_println("Running sessions list:")?;
            let names = ctx.list_running_entries().await;
            if names.is_empty() {
                info!("sessions none");
                ui_println("no running sessions")?;
            } else {
                info!(count = names.len(), "sessions listed");
                for n in names {
                    ui_println(&format!("  {n}"))?;
                }
            }
        }
        ["default", "set", name] => match ctx.set_default_mode(name) {
            true => info!(name = *name, "set_default_shell ok"),
            false => warn!(name = *name, "set_default_shell unknown"),
        },
        ["default", "get"] => match ctx.get_default_mode() {
            Some(n) => info!(name = %n, "shell_default"),
            None => warn!("shell_default unset"),
        },
        _ => {
            warn!(args = args, "admin_unrecognized");
            return Err(BuiltinError::AdminUnrecognized {
                args: args.to_string(),
            }
            .into());
        }
    }

    info!("builtin_admin_handle ok");
    Ok(())
}

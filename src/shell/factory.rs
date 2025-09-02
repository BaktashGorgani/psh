use tracing::{debug, info, instrument};

use crate::{
    error::Result,
    shell::{PtyShell, ShellSpec},
};

#[instrument(fields(name = %name, kind = ?spec, cols, rows))]
pub async fn spawn(
    name: &str,
    spec: &ShellSpec,
    cols: u16,
    rows: u16,
) -> Result<PtyShell> {
    debug!("shell_factory_spawn start");
    let shell = match spec {
        ShellSpec::Local { program } => {
            PtyShell::spawn(name, program, &[], cols, rows).await
        }
        ShellSpec::Remote {
            target,
            port,
            extra_args,
        } => {
            let mut argv: Vec<String> = vec!["-tt".to_string()];
            if let Some(p) = port.as_ref() {
                argv.push("-p".to_string());
                argv.push(p.to_string());
            }
            argv.extend(extra_args.iter().cloned());
            argv.push(target.clone());
            let owned = argv;
            let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
            PtyShell::spawn(name, "ssh", &refs, cols, rows).await
        }
    }?;
    info!("shell_factory_spawn ok");
    Ok(shell)
}

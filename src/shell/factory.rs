use tracing::{debug, info, instrument};

use crate::{
    error::Result,
    shell::{PtyShell, ShellSpec, spec::RemoteBackend},
};

const SSH_PROGRAM: &str = "ssh";
const SSH_PTY_FLAG: &str = "-tt";
const SSH_PORT_FLAG: &str = "-p";
const TELNET_PROGRAM: &str = "telnet";

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
        ShellSpec::Remote { host, backend } => match backend {
            RemoteBackend::Ssh { port, extra_args } => {
                let mut argv: Vec<String> = vec![SSH_PTY_FLAG.to_string()];
                argv.push(SSH_PORT_FLAG.to_string());
                argv.push(port.to_string());
                argv.extend(extra_args.iter().cloned());
                argv.push(host.clone());
                let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
                PtyShell::spawn(name, SSH_PROGRAM, &refs, cols, rows).await
            }
            RemoteBackend::Telnet { port, extra_args } => {
                let mut argv: Vec<String> = extra_args.clone();
                argv.push(host.clone());
                argv.push(port.to_string());
                let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
                PtyShell::spawn(name, TELNET_PROGRAM, &refs, cols, rows).await
            }
        },
    }?;
    info!("shell_factory_spawn ok");
    Ok(shell)
}

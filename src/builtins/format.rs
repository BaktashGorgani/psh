use tracing::{debug, info};

use crate::shell::{ShellSpec, spec::RemoteBackend};

pub fn format_shell_line(name: &str, spec: &ShellSpec, running: bool) -> String {
    debug!(
        name = name,
        running = running,
        shell = format!("{:?}", spec),
        "format_shell_line start"
    );
    let s = match spec {
        ShellSpec::Local { program } => {
            let status = if running { "[running]" } else { "[stopped]" };
            format!("  {name}: {program} {status}")
        }
        ShellSpec::Remote { host, backend } => {
            let status = if running {
                "[connected]"
            } else {
                "[disconnected]"
            };
            match backend {
                RemoteBackend::Ssh { port, .. } => {
                    format!("  {name} (ssh): {host}:{port} {status}")
                }
                RemoteBackend::Telnet { port, .. } => {
                    format!("  {name} (telnet): {host}:{port} {status}")
                }
            }
        }
    };
    info!("format_shell_line ok");
    s
}

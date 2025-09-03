use tracing::{debug, info};

use crate::shell::ShellSpec;

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
        ShellSpec::Remote { target, port, .. } => {
            let status = if running {
                "[connected]"
            } else {
                "[disconnected]"
            };
            let mut line = format!("  {name}: {target}");
            match port {
                Some(p) => line.push_str(&format!(":{p}")),
                None => line.push_str(":22"),
            }
            line.push(' ');
            line.push_str(status);
            line
        }
    };
    info!("format_shell_line ok");
    s
}

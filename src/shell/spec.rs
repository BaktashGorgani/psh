use serde::{Deserialize, Serialize};
use tracing::{debug, info};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "lowercase")]
pub enum RemoteBackend {
    Ssh {
        #[serde(default = "default_ssh_port")]
        port: u16,
        #[serde(default)]
        extra_args: Vec<String>,
    },
    Telnet {
        #[serde(default = "default_telnet_port")]
        port: u16,
        #[serde(default)]
        extra_args: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ShellSpec {
    Local {
        program: String,
    },
    Remote {
        target: String,
        #[serde(flatten)]
        backend: RemoteBackend,
    },
}

impl ShellSpec {
    pub fn kind_name(&self) -> &'static str {
        debug!("shellspec_kind_name start");
        let k = match self {
            ShellSpec::Local { .. } => "local",
            ShellSpec::Remote { backend, .. } => match backend {
                RemoteBackend::Ssh { .. } => "remote_ssh",
                RemoteBackend::Telnet { .. } => "remote_telnet",
            },
        };
        info!("shellspec_kind_name ok");
        k
    }
}

fn default_ssh_port() -> u16 {
    22
}

fn default_telnet_port() -> u16 {
    23
}

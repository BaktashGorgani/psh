use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ShellSpec {
    Local {
        program: String,
    },
    Remote {
        target: String,
        #[serde(default)]
        port: Option<u16>,
        #[serde(default)]
        extra_args: Vec<String>,
    },
}

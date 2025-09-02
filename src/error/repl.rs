use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReplRouterError {
    #[error("unknown shell: {name}")]
    UnknownShell { name: String },

    #[error("default shell is not set")]
    DefaultShellUnset,

    #[error("default shell is unknown: {name}")]
    DefaultShellUnknown { name: String },

    #[error("session is not running: {name}")]
    SessionNotRunning { name: String },
}

#[derive(Debug, Error)]
pub enum ReplError {
    #[error(transparent)]
    Router(#[from] ReplRouterError),
}

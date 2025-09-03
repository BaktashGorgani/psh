use thiserror::Error;

pub mod builtin;
pub mod repl;
pub mod runtime;
pub mod shell;
pub mod sync;
pub mod ui;

pub use builtin::BuiltinError;
pub use repl::{ReplError, ReplRouterError};
pub use runtime::RuntimeError;
pub use shell::ShellError;
pub use sync::SyncError;
pub use ui::UiError;

#[derive(Debug, Error)]
pub enum PshError {
    #[error(transparent)]
    Builtin(#[from] BuiltinError),

    #[error(transparent)]
    Repl(#[from] ReplError),

    #[error(transparent)]
    Runtime(#[from] RuntimeError),

    #[error(transparent)]
    Shell(#[from] ShellError),

    #[error(transparent)]
    Ui(#[from] UiError),
}

pub type Result<T> = std::result::Result<T, PshError>;

impl From<ReplRouterError> for PshError {
    fn from(e: ReplRouterError) -> Self {
        PshError::Repl(ReplError::from(e))
    }
}

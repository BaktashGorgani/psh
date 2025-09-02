use anyhow::Error as AnyError;
use tokio::task::JoinError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShellError {
    #[error("failed to open PTY")]
    PtyOpen(#[source] AnyError),

    #[error("failed to spawn child")]
    Spawn(#[source] AnyError),

    #[error("failed to clone PTY reader")]
    CloneReader(#[source] AnyError),

    #[error("failed to clone PTY writer")]
    TakeWriter(#[source] AnyError),

    #[error("failed to write to PTY")]
    Write(#[source] AnyError),

    #[error("failed to read from PTY")]
    Read(#[source] AnyError),

    #[error("failed to resize PTY")]
    Resize(#[source] AnyError),

    #[error("failed to wait on child")]
    Wait(#[source] AnyError),

    #[error("background task join failed")]
    Join(#[source] JoinError),

    #[error("command channel closed")]
    ChannelClosed,
}

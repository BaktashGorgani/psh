use std::io::Error as IoError;

use anyhow::Error as AnyError;
use thiserror::Error;
use tokio::sync::broadcast::error::RecvError as TokioRecvError;

#[derive(Debug, Error)]
pub enum UiError {
    #[error("stdin read failed")]
    IoRead(#[source] IoError),

    #[error("stdin write failed")]
    IoWrite(#[source] IoError),

    #[error("terminal size read failed")]
    ResizeRead(#[source] IoError),

    #[error("failed to enable raw mode")]
    RawModeEnable(#[source] AnyError),

    #[error("failed to disable raw mode")]
    RawModeDisable(#[source] AnyError),

    #[error("event receive failed")]
    EventRecv(#[source] TokioRecvError),
}

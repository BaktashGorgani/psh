use anyhow::Error as AnyError;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("mutex poisoned: {context}")]
    MutexPoison { context: String },

    #[error("channel send failed")]
    ChannelSend(#[source] AnyError),

    #[error("channel receive failed")]
    ChannelRecv(#[source] AnyError),

    #[error("channel closed: {context}")]
    ChannelClosed { context: String },

    #[error("background task join failed")]
    Join(#[source] JoinError),
}

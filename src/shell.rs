use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::error::Result;

pub mod cmd;
pub mod event;
pub mod factory;
pub mod pty;
pub mod spec;

#[cfg(feature = "mock-shell")]
pub mod mock;

pub use cmd::ShellCmd;
pub use event::ShellEvent;
pub use pty::PtyShell;
pub use spec::ShellSpec;

#[async_trait]
pub trait Shell: Send + Sync {
    async fn send_line(&self, line: String) -> Result<()>;
    async fn send_bytes(&self, bytes: Vec<u8>) -> Result<()>;
    async fn resize(&self, cols: u16, rows: u16) -> Result<()>;
    async fn shutdown(&self) -> Result<()>;
    fn subscribe(&self) -> broadcast::Receiver<ShellEvent>;
}

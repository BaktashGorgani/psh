#![cfg(feature = "mock-shell")]

use std::sync::Arc;

use async_trait::async_trait;
use tokio::{
    sync::{broadcast, mpsc},
    task,
};
use tracing::{debug, info};

use crate::{
    error::{Result, ShellError},
    shell::{Shell, ShellEvent},
};

pub struct MockShell {
    name: String,
    tx: mpsc::Sender<Vec<u8>>,
    events: broadcast::Sender<ShellEvent>,
}

#[async_trait]
impl Shell for MockShell {
    async fn send_line(&self, line: String) -> Result<()> {
        debug!(shell = %self.name, %line, "mock_send_line start");
        self.tx
            .send(line.into_bytes())
            .await
            .map_err(|_| ShellError::ChannelClosed)?;
        info!(shell = %self.name, "mock_send_line ok");
        Ok(())
    }

    async fn send_bytes(&self, bytes: Vec<u8>) -> Result<()> {
        debug!(shell = %self.name, size = bytes.len(), "mock_send_bytes start");
        self.tx
            .send(bytes)
            .await
            .map_err(|_| ShellError::ChannelClosed)?;
        info!(shell = %self.name, "mock_send_bytes ok");
        Ok(())
    }

    async fn resize(&self, _cols: u16, _rows: u16) -> Result<()> {
        debug!(shell = %self.name, "mock_resize start");
        info!(shell = %self.name, "mock_resize ok");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        debug!(shell = %self.name, "mock_shutdown start");
        let _ = self.events.send(ShellEvent::Exited("mock_shutdown".into()));
        info!(shell = %self.name, "mock_shutdown ok");
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<ShellEvent> {
        debug!(shell = %self.name, "mock_subscribe start");
        let rx = self.events.subscribe();
        info!(shell = %self.name, "mock_subscribe ok");
        rx
    }
}

impl MockShell {
    pub fn spawn(name: &str) -> Result<Arc<Self>> {
        debug!(shell = name, "mock_spawn start");
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);
        let (ev_tx, _) = broadcast::channel::<ShellEvent>(1024);
        let name_owned = name.to_string();
        let ev_tx_cl = ev_tx.clone();
        task::spawn(async move {
            info!(shell = %name_owned, "mock_worker start");
            while let Some(bytes) = rx.recv().await {
                let s = String::from_utf8_lossy(&bytes).to_string();
                let _ = ev_tx_cl.send(ShellEvent::Output(s));
            }
            info!(shell = %name_owned, "mock_worker done");
        });
        let s = Arc::new(Self {
            name: name.to_string(),
            tx,
            events: ev_tx,
        });
        info!(shell = name, "mock_spawn ok");
        Ok(s)
    }
}

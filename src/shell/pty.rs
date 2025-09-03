use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use tokio::{
    sync::{broadcast, mpsc},
    task,
};
use tracing::{debug, error, info, warn};

use crate::{
    error::{Result, ShellError},
    shell::{Shell, ShellCmd, ShellEvent},
};

const SHELL_CMD_CHANNEL_CAP: usize = 64;
const SHELL_EVENT_CHANNEL_CAP: usize = 1024;
const PTY_READ_BUF_SIZE: usize = 4096;
const NEWLINE: &[u8] = b"\n";
const SHELL_EXIT_CMD: &[u8] = b"exit\n";

pub struct PtyShell {
    name: String,
    tx: mpsc::Sender<ShellCmd>,
    events: broadcast::Sender<ShellEvent>,
}

#[async_trait]
impl Shell for PtyShell {
    async fn send_line(&self, line: String) -> Result<()> {
        debug!(shell = %self.name, %line, "send_line");
        self.tx
            .send(ShellCmd::WriteLine(line))
            .await
            .map_err(|_| ShellError::ChannelClosed)?;
        info!(shell = %self.name, "send_line ok");
        Ok(())
    }

    async fn send_bytes(&self, bytes: Vec<u8>) -> Result<()> {
        debug!(shell = %self.name, size = bytes.len(), "send_bytes");
        self.tx
            .send(ShellCmd::WriteBytes(bytes))
            .await
            .map_err(|_| ShellError::ChannelClosed)?;
        info!(shell = %self.name, "send_bytes ok");
        Ok(())
    }

    async fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        debug!(shell = %self.name, cols, rows, "resize");
        self.tx
            .send(ShellCmd::Resize(cols, rows))
            .await
            .map_err(|_| ShellError::ChannelClosed)?;
        info!(shell = %self.name, "resize ok");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        debug!(shell = %self.name, "shutdown");
        self.tx
            .send(ShellCmd::Shutdown)
            .await
            .map_err(|_| ShellError::ChannelClosed)?;
        info!(shell = %self.name, "shutdown ok");
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<ShellEvent> {
        debug!(shell = %self.name, "subscribe");
        let rx = self.events.subscribe();
        info!(shell = %self.name, "subscribe ok");
        rx
    }
}

impl PtyShell {
    pub async fn spawn(
        name: &str,
        program: &str,
        args: &[&str],
        cols: u16,
        rows: u16,
    ) -> Result<Self> {
        debug!(shell = name, program = program, cols, rows, "spawn start");
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(ShellError::PtyOpen)?;
        let mut cmd = CommandBuilder::new(program);
        cmd.args(args.iter());
        let mut child = pair.slave.spawn_command(cmd).map_err(ShellError::Spawn)?;
        let master = pair.master;
        drop(pair.slave);
        let reader = master.try_clone_reader().map_err(ShellError::CloneReader)?;
        let writer = master.take_writer().map_err(ShellError::TakeWriter)?;
        let master_arc: Arc<Mutex<Box<dyn MasterPty + Send>>> =
            Arc::new(Mutex::new(master));
        let writer_arc: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(writer));
        let (tx, mut rx) = mpsc::channel::<ShellCmd>(SHELL_CMD_CHANNEL_CAP);
        let (ev_tx, _) = broadcast::channel::<ShellEvent>(SHELL_EVENT_CHANNEL_CAP);

        let reader_name = name.to_string();
        let ev_tx_reader = ev_tx.clone();
        task::spawn_blocking(move || {
            info!(shell = %reader_name, "reader started");
            let mut r = reader;
            let mut buf = [0u8; PTY_READ_BUF_SIZE];
            loop {
                match r.read(&mut buf) {
                    Ok(0) => {
                        info!(shell = %reader_name, "reader eof");
                        if ev_tx_reader
                            .send(ShellEvent::Exited("eof".to_string()))
                            .is_err()
                        {
                            warn!(shell = %reader_name, "no subscribers for exit");
                        }
                        break;
                    }
                    Ok(n) => {
                        let s = String::from_utf8_lossy(&buf[..n]).to_string();
                        info!(shell = %reader_name, bytes = n, "read chunk");
                        let _ = ev_tx_reader.send(ShellEvent::Output(s));
                    }
                    Err(e) => {
                        error!(shell = %reader_name, ?e, "reader error");
                        let _ = ev_tx_reader
                            .send(ShellEvent::Exited(format!("reader_error: {e}")));
                        break;
                    }
                }
            }
            info!(shell = %reader_name, "reader done");
        });

        let writer_name = name.to_string();
        let ev_tx_writer = ev_tx.clone();
        let writer_arc_task = writer_arc.clone();
        let master_arc_task = master_arc.clone();
        tokio::spawn(async move {
            info!(shell = %writer_name, "writer started");
            while let Some(msg) = rx.recv().await {
                match msg {
                    ShellCmd::WriteLine(line) => {
                        debug!(shell = %writer_name, %line, "write requested");
                        let wa = writer_arc_task.clone();
                        let res = task::spawn_blocking(move || {
                            let mut w = wa.lock().unwrap();
                            w.write_all(line.as_bytes())?;
                            w.write_all(NEWLINE)?;
                            w.flush()
                        })
                        .await;
                        match res {
                            Ok(Ok(())) => info!(shell = %writer_name, "write ok"),
                            Ok(Err(e)) => {
                                error!(shell = %writer_name, ?e, "write failed");
                                let _ = ev_tx_writer.send(ShellEvent::Exited(format!(
                                    "write_failed: {e}"
                                )));
                                break;
                            }
                            Err(e) => {
                                error!(shell = %writer_name, ?e, "join error");
                                let _ = ev_tx_writer.send(ShellEvent::Exited(format!(
                                    "join_failed: {e}"
                                )));
                                break;
                            }
                        }
                    }
                    ShellCmd::WriteBytes(bytes) => {
                        debug!(shell = %writer_name, size = bytes.len(), "write_bytes requested");
                        let wa = writer_arc_task.clone();
                        let res = task::spawn_blocking(move || {
                            let mut w = wa.lock().unwrap();
                            w.write_all(&bytes)?;
                            w.flush()
                        })
                        .await;
                        match res {
                            Ok(Ok(())) => info!(shell = %writer_name, "write_bytes ok"),
                            Ok(Err(e)) => {
                                error!(shell = %writer_name, ?e, "write_bytes failed");
                                let _ = ev_tx_writer.send(ShellEvent::Exited(format!(
                                    "write_bytes_failed: {e}"
                                )));
                                break;
                            }
                            Err(e) => {
                                error!(shell = %writer_name, ?e, "write_bytes join error");
                                let _ = ev_tx_writer.send(ShellEvent::Exited(format!(
                                    "write_bytes_join_failed: {e}"
                                )));
                                break;
                            }
                        }
                    }
                    ShellCmd::Resize(cols, rows) => {
                        debug!(shell = %writer_name, cols, rows, "resize requested");
                        let ma = master_arc_task.clone();
                        let res = task::spawn_blocking(move || {
                            let m = ma.lock().unwrap();
                            m.resize(PtySize {
                                rows,
                                cols,
                                pixel_width: 0,
                                pixel_height: 0,
                            })
                        })
                        .await;
                        match res {
                            Ok(Ok(())) => info!(shell = %writer_name, "resize ok"),
                            Ok(Err(e)) => {
                                error!(shell = %writer_name, ?e, "resize failed")
                            }
                            Err(e) => {
                                error!(shell = %writer_name, ?e, "resize join failed")
                            }
                        }
                    }
                    ShellCmd::Shutdown => {
                        info!(shell = %writer_name, "shutdown requested");
                        let wa = writer_arc_task.clone();
                        let _ = task::spawn_blocking(move || {
                            let mut w = wa.lock().unwrap();
                            w.write_all(SHELL_EXIT_CMD)?;
                            w.flush()
                        })
                        .await;
                        break;
                    }
                }
            }
            info!(shell = %writer_name, "writer done")
        });

        let wait_name = name.to_string();
        let ev_tx_wait = ev_tx.clone();
        task::spawn_blocking(move || {
            info!(shell = %wait_name, "waiter started");
            match child.wait() {
                Ok(status) => {
                    info!(shell = %wait_name, status = format!("{status:?}"), "child exited");
                    let _ = ev_tx_wait.send(ShellEvent::Exited(format!("{status:?}")));
                }
                Err(e) => {
                    error!(shell = %wait_name, ?e, "wait failed");
                    let _ = ev_tx_wait
                        .send(ShellEvent::Exited(format!("wait_failed: {e}")));
                }
            }
            info!(shell = %wait_name, "waiter done");
        });

        info!(shell = name, "spawn ok");
        Ok(Self {
            name: name.to_string(),
            tx,
            events: ev_tx,
        })
    }
}

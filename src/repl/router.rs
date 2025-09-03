use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_trait::async_trait;
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument, warn};

use crate::{
    builtins::{self, BuiltinContext},
    error::{ReplRouterError, Result},
    registry::{self, Registry},
    repl::parser::{self, Parsed},
    shell::{PtyShell, Shell, ShellEvent, ShellSpec, factory},
};

pub struct Router {
    registry: Registry,
    sessions: Arc<Mutex<HashMap<String, Arc<PtyShell>>>>,
    default_shell: Option<String>,
    cols: u16,
    rows: u16,
}

impl Router {
    pub fn new(
        registry: Registry,
        default_shell: Option<String>,
        cols: u16,
        rows: u16,
    ) -> Self {
        debug!(cols, rows, "router_new start");
        let s = Self {
            registry,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            default_shell,
            cols,
            rows,
        };
        info!("router_new ok");
        s
    }

    pub fn parse_preview(&self, input: &str) -> Parsed {
        debug!(input = input, "parse_preview start");
        let p = parser::parse(&self.registry, input);
        info!("parse_preview ok");
        p
    }

    #[instrument(skip(self, input), fields(input_len = input.len()))]
    pub async fn exec(&mut self, input: &str) -> Result<()> {
        debug!(input = input, "router_exec start");
        match parser::parse(&self.registry, input) {
            Parsed::Entry {
                name,
                entry,
                command,
            } => match entry {
                registry::Entry::Shell(spec) => {
                    let s = self.ensure_shell_session_by_spec(&name, &spec).await?;
                    s.send_line(command).await?;
                    info!(mode = %name, "router_exec shell ok");
                }
                registry::Entry::Builtin => match name.as_str() {
                    "local" => builtins::local::handle(self, &command).await?,
                    "remote" => builtins::remote::handle(self, &command).await?,
                    "admin" => builtins::admin::handle(self, &command).await?,
                    "quit" | "exit" => builtins::quit::handle(self, &command).await?,
                    other => warn!(builtin = other, "router_exec builtin unknown"),
                },
            },
            Parsed::Default { command } => {
                let Some(default_name) = self.default_shell.clone() else {
                    warn!("router_exec no_default_shell");
                    return Err(ReplRouterError::DefaultShellUnset.into());
                };
                let Some(spec) = self.registry.get_shell_spec(&default_name) else {
                    warn!(name = %default_name, "router_exec unknown_default");
                    return Err(ReplRouterError::DefaultShellUnknown {
                        name: default_name,
                    }
                    .into());
                };
                let s = self
                    .ensure_shell_session_by_spec(&default_name, &spec)
                    .await?;
                s.send_line(command).await?;
                info!(mode = %default_name, "router_exec default->shell ok");
            }
        }
        info!("router_exec ok");
        Ok(())
    }

    pub async fn add_and_start_shell(
        &mut self,
        name: &str,
        spec: ShellSpec,
    ) -> Result<()> {
        debug!(
            name = name,
            entry = format!("{:?}", spec),
            "add_and_start_shell start"
        );
        self.ensure_shell_session_by_spec(name, &spec).await?;
        self.registry
            .register_entry(name.to_string(), registry::Entry::Shell(spec));
        info!(name = name, "add_and_start_shell ok");
        Ok(())
    }

    pub async fn stop_shell_session(&mut self, name: &str) -> Result<()> {
        debug!(name = name, "stop_shell_session start");
        let opt = {
            let mut map = self.sessions.lock().await;
            map.remove(name)
        };
        match opt {
            Some(s) => {
                if let Err(e) = s.shutdown().await {
                    warn!(name = name, ?e, "stop_shell_session shutdown failed");
                }
                info!(name = name, "stop_shell_session ok");
                Ok(())
            }
            None => {
                warn!(name = name, "stop_shell_session not_running");
                Err(ReplRouterError::SessionNotRunning {
                    name: name.to_string(),
                }
                .into())
            }
        }
    }

    pub async fn ensure_shell_session_by_name(
        &mut self,
        name: &str,
    ) -> Result<Arc<PtyShell>> {
        debug!(name = name, "ensure_shell_session_by_name start");
        let Some(spec) = self.registry.get_shell_spec(name) else {
            error!(name = name, "ensure_shell_session_by_name unknown");
            return Err(ReplRouterError::UnknownShell {
                name: name.to_string(),
            }
            .into());
        };
        let s = self.ensure_shell_session_by_spec(name, &spec).await?;
        info!(name = name, "ensure_shell_session_by_name ok");
        Ok(s)
    }

    pub async fn ensure_shell_session_by_spec(
        &mut self,
        name: &str,
        spec: &ShellSpec,
    ) -> Result<Arc<PtyShell>> {
        debug!(
            name = name,
            kind = format!("{:?}", spec),
            "ensure_shell_session_by_spec start"
        );

        {
            let map = self.sessions.lock().await;
            if let Some(s) = map.get(name) {
                info!(name = name, "ensure_shell_session_by_spec hit");
                return Ok(s.clone());
            }
        }

        let s = factory::spawn(name, spec, self.cols, self.rows).await?;
        let s = Arc::new(s);

        {
            let mut map = self.sessions.lock().await;
            map.insert(name.to_string(), s.clone());
            info!(count = map.len(), "sessions count after insert");
        }

        info!(name = name, "ensure_shell_session_by_spec miss_created");

        let mut rx = s.subscribe();
        let sessions_arc = self.sessions.clone();
        let name_owned = name.to_string();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(ShellEvent::Exited(reason)) => {
                        let mut map = sessions_arc.lock().await;
                        if map.remove(&name_owned).is_some() {
                            info!(name = %name_owned, %reason, count = map.len(), "session removed (auto-cleanup)");
                        } else {
                            warn!(name = %name_owned, %reason, "session not found for removal");
                        }
                        break;
                    }
                    Ok(ShellEvent::Output(_)) => {}
                    Err(e) => {
                        warn!(?e, name = %name_owned, "event recv failed in watcher");
                        break;
                    }
                }
            }
            info!(name = %name_owned, "watcher done");
        });

        Ok(s)
    }

    pub async fn list_entries_with_status(
        &self,
    ) -> Vec<(String, registry::Entry, bool)> {
        debug!("list_entries_with_status start");
        let running = {
            let map = self.sessions.lock().await;
            map.keys().cloned().collect::<HashSet<_>>()
        };
        let mut v: Vec<(String, registry::Entry, bool)> = self
            .registry
            .list_entries()
            .into_iter()
            .map(|(name, entry)| {
                let is_running = running.contains(&name);
                (name, entry, is_running)
            })
            .collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        info!(count = v.len(), "list_entries_with_status ok");
        v
    }

    pub async fn list_running_entries(&self) -> Vec<String> {
        debug!("list_running_shell_names start");
        let map = self.sessions.lock().await;
        let mut names: Vec<String> = map.keys().cloned().collect();
        names.sort();
        info!(count = names.len(), "list_running_shell_names ok");
        names
    }

    pub fn get_registry_clone(&self) -> Registry {
        debug!("get_registry_clone start");
        let r = self.registry.clone();
        info!("get_registry_clone ok");
        r
    }

    pub fn register_entry(&mut self, name: String, entry: registry::Entry) {
        debug!(name = %name, entry = format!("{:?}", entry), "router_register_entry start");
        self.registry.register_entry(name, entry);
        info!("router_register_entry ok")
    }

    pub fn unregister_entry(&mut self, name: &str) {
        debug!(name = name, "router_unregister_entry start");
        self.registry.unregister_entry(name);
        info!("router_unregister_entry ok")
    }

    pub fn set_default_shell(&mut self, name: &str) -> bool {
        debug!(name = name, "set_default_shell start");
        match self.registry.get_shell_spec(name) {
            Some(_) => {
                self.default_shell = Some(name.to_string());
                info!(name = name, "set_default_shell ok");
                true
            }
            None => {
                warn!(name = name, "set_default_shell unknown");
                false
            }
        }
    }

    pub fn get_default_shell(&self) -> Option<String> {
        debug!("get_default_shell start");
        let r = self.default_shell.clone();
        let present = r.is_some();
        info!(present = present, "get_default_shell ok");
        r
    }
}

#[async_trait]
impl BuiltinContext for Router {
    async fn add_and_start_shell(
        &mut self,
        name: String,
        spec: ShellSpec,
    ) -> Result<()> {
        Router::add_and_start_shell(self, &name, spec).await
    }

    async fn stop_shell_session(&mut self, name: &str) -> Result<()> {
        Router::stop_shell_session(self, name).await
    }

    async fn ensure_shell_session_by_name(
        &mut self,
        name: &str,
    ) -> Result<Arc<PtyShell>> {
        Router::ensure_shell_session_by_name(self, name).await
    }

    async fn list_entries_with_status(&self) -> Vec<(String, registry::Entry, bool)> {
        Router::list_entries_with_status(self).await
    }

    async fn list_running_entries(&self) -> Vec<String> {
        Router::list_running_entries(self).await
    }

    fn list_entries(&self) -> Vec<(String, registry::Entry)> {
        self.registry.list_entries()
    }

    fn register_entry(&mut self, name: String, entry: registry::Entry) {
        Router::register_entry(self, name, entry);
    }

    fn unregister_entry(&mut self, name: &str) {
        Router::unregister_entry(self, name);
    }

    fn set_default_shell(&mut self, name: &str) -> bool {
        Router::set_default_shell(self, name)
    }

    fn get_default_shell(&self) -> Option<String> {
        Router::get_default_shell(self)
    }
}

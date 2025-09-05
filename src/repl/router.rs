use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_trait::async_trait;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::{
    builtins::{self, BuiltinContext},
    error::{ReplRouterError, Result},
    registry::{self, Registry},
    repl::{
        ModeState,
        parser::{self, Parsed},
    },
    shell::{PtyShell, Shell, ShellEvent, ShellSpec, factory},
};

pub struct Router {
    registry: Registry,
    mode: ModeState,
    sessions: Arc<Mutex<HashMap<String, Arc<PtyShell>>>>,
    cols: u16,
    rows: u16,
}

impl Router {
    pub fn new(registry: Registry, cols: u16, rows: u16) -> Self {
        debug!(cols, rows, "router_new start");
        let s = Self {
            registry,
            mode: ModeState::default(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
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

    pub fn get_registry_clone(&self) -> Registry {
        debug!("get_registry_clone start");
        let r = self.registry.clone();
        info!("get_registry_clone ok");
        r
    }

    pub fn mode_state(&self) -> ModeState {
        debug!("router_mode_state start");
        let m = self.mode.clone();
        info!("router_mode_state ok");
        m
    }

    pub fn get_current_mode(&self) -> Option<String> {
        debug!("get_current_mode_name start");
        let r = self.mode.get_current();
        let present = r.is_some();
        info!(present = present, "get_current_mode_name ok");
        r
    }

    pub fn set_current_mode(&self, name: &str) -> bool {
        debug!(name = name, "set_current_mode start");
        if self.registry.has_entry(name) {
            self.mode.set_current(Some(name.to_string()));
            info!(name = name, "set_current_mode ok");
            true
        } else {
            warn!(name = name, "set_current_mode unknown");
            false
        }
    }

    pub fn get_default_mode(&self) -> Option<String> {
        debug!("get_default_mode start");
        let r = self.mode.get_default();
        let present = r.is_some();
        info!(present = present, "get_default_mode ok");
        r
    }

    pub fn set_default_mode(&self, name: &str) -> bool {
        debug!(name = name, "set_default_mode start");
        if self.registry.has_entry(name) {
            self.mode.set_default(Some(name.to_string()));
            info!(name = name, "set_default_mode ok");
            true
        } else {
            warn!(name = name, "set_default_mode unknown");
            false
        }
    }

    async fn exec_by_prefix(&mut self, name: &str, command: &str) -> Result<()> {
        debug!(name = name, "exec_by_prefix start");
        match self.registry.get_entry(name) {
            Some(registry::Entry::Shell(spec)) => {
                let s = self.ensure_shell_session_by_spec(name, &spec).await?;
                s.send_line(command.to_string()).await?;
                info!(name = name, "exec_by_prefix shell ok");
            }
            Some(registry::Entry::Builtin) => match name {
                "local" => builtins::local::handle(self, command).await?,
                "remote" => builtins::remote::handle(self, command).await?,
                "admin" => builtins::admin::handle(self, command).await?,
                "quit" | "exit" => builtins::quit::handle(self, command).await?,
                other => warn!(builtin = other, "exec_by_prefix builtin unknown,"),
            },
            None => {
                warn!(name = name, "exec_by_prefix unknown");
                return Err(ReplRouterError::UnknownShell {
                    name: name.to_string(),
                }
                .into());
            }
        }
        info!("exec_by_prefix ok");
        Ok(())
    }

    pub async fn exec(&mut self, input: &str) -> Result<()> {
        debug!(input = input, "router_exec start");
        match parser::parse(&self.registry, input) {
            Parsed::Entry { name, command, .. } => {
                self.set_current_mode(&name);
                self.exec_by_prefix(&name, &command).await?;
            }
            Parsed::Default { command } => {
                let target =
                    self.mode.get_current().or_else(|| self.mode.get_default());
                match target {
                    Some(name) => self.exec_by_prefix(&name, &command).await?,
                    None => {
                        warn!("router_exec no_current_or_default");
                        return Err(ReplRouterError::DefaultShellUnset.into());
                    }
                }
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

    fn get_current_mode(&self) -> Option<String> {
        Router::get_current_mode(&self)
    }

    fn set_current_mode(&mut self, name: &str) -> bool {
        Router::set_current_mode(&self, name)
    }

    fn get_default_mode(&self) -> Option<String> {
        Router::get_current_mode(&self)
    }

    fn set_default_mode(&mut self, name: &str) -> bool {
        Router::set_default_mode(&self, name)
    }
}

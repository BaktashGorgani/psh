use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    error::Result,
    registry::Entry,
    shell::{PtyShell, ShellSpec},
};

pub mod admin;
pub mod local;
pub mod quit;
pub mod remote;

#[async_trait]
pub trait BuiltinContext: Send {
    async fn add_and_start_shell(
        &mut self,
        name: String,
        spec: ShellSpec,
    ) -> Result<()>;
    async fn stop_shell_session(&mut self, name: &str) -> Result<()>;
    async fn ensure_shell_session_by_name(
        &mut self,
        name: &str,
    ) -> Result<Arc<PtyShell>>;
    async fn list_entries_with_status(&self) -> Vec<(String, Entry, bool)>;
    async fn list_running_entries(&self) -> Vec<String>;

    fn list_entries(&self) -> Vec<(String, Entry)>;
    fn register_entry(&mut self, name: String, entry: Entry);
    fn unregister_entry(&mut self, name: &str);

    fn set_default_shell(&mut self, name: &str) -> bool;
    fn get_default_shell(&self) -> Option<String>;
}

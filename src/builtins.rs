use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    error::Result,
    registry,
    shell::{PtyShell, ShellSpec},
};

pub mod admin;
pub mod format;
pub mod local;
pub mod quit;
pub mod remote;

pub use format::format_shell_line;

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
    async fn list_entries_with_status(&self) -> Vec<(String, registry::Entry, bool)>;
    async fn list_running_entries(&self) -> Vec<String>;

    fn list_entries(&self) -> Vec<(String, registry::Entry)>;
    fn register_entry(&mut self, name: String, entry: registry::Entry);
    fn unregister_entry(&mut self, name: &str);

    fn get_current_mode(&self) -> Option<String>;
    fn set_current_mode(&mut self, name: &str) -> bool;

    fn get_default_mode(&self) -> Option<String>;
    fn set_default_mode(&mut self, name: &str) -> bool;
}

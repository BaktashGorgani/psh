use std::collections::HashMap;

use tracing::{debug, info, warn};

use crate::shell::ShellSpec;

#[derive(Debug, Clone)]
pub enum Entry {
    Shell(ShellSpec),
    Builtin,
}

#[derive(Debug, Default, Clone)]
pub struct Registry {
    entries: HashMap<String, Entry>,
    max_len: usize,
}

impl Registry {
    pub fn new() -> Self {
        debug!("registry_new start");
        let s = Self {
            entries: HashMap::new(),
            max_len: 0,
        };
        info!("registry_new ok");
        s
    }

    pub fn with_builtins() -> Self {
        debug!("registry_with_builtins start");
        let mut r = Self::new();
        r.register_entry("local", Entry::Builtin);
        r.register_entry("remote", Entry::Builtin);
        r.register_entry("admin", Entry::Builtin);
        r.register_entry("quit", Entry::Builtin);
        r.register_entry("exit", Entry::Builtin);
        info!("registry_with_builtins ok");
        r
    }

    pub fn register_entry(&mut self, name: impl Into<String>, entry: Entry) {
        let name = name.into();
        debug!(name = %name, entry = format!("{:?}", entry), "register_entry start");
        if self.entries.contains_key(&name) {
            warn!(name = %name, "register_entry duplicate");
        }
        self.entries.insert(name.clone(), entry);
        self.recompute_max_name_len();
        info!(
            count = self.entries.len(),
            max_len = self.max_name_len(),
            "register_entry ok"
        );
    }

    pub fn unregister_entry(&mut self, name: &str) {
        debug!(name = name, "unregister_entry start");
        if self.entries.remove(name).is_some() {
            self.recompute_max_name_len();
            info!(name = name, "unregister_entry ok");
        } else {
            warn!(name = name, "unregister_entry not_found");
        }
    }

    pub fn has_entry(&self, name: &str) -> bool {
        debug!(name = name, "has_entry start");
        let r = self.entries.contains_key(name);
        info!(name = name, present = r, "has_entry ok");
        r
    }

    pub fn get_entry(&self, name: &str) -> Option<Entry> {
        debug!(name = name, "get_entry start");
        let r = self.entries.get(name).cloned();
        let present = r.is_some();
        info!(name = name, present = present, "get_entry ok");
        r
    }

    pub fn list_entries(&self) -> Vec<(String, Entry)> {
        debug!("list_entries start");
        let mut v: Vec<_> = self
            .entries
            .iter()
            .map(|(k, e)| (k.clone(), e.clone()))
            .collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        info!(count = v.len(), "list_entries ok");
        v
    }

    pub fn get_shell_spec(&self, name: &str) -> Option<ShellSpec> {
        debug!(name = name, "get_shell_spec start");
        let r = match self.entries.get(name) {
            Some(Entry::Shell(spec)) => Some(spec.clone()),
            _ => None,
        };
        let present = r.is_some();
        info!(name = name, present = present, "get_shell_spec ok");
        r
    }
    pub fn max_name_len(&self) -> usize {
        debug!(len = self.max_len, "max_name_len start");
        info!(len = self.max_len, "max_name_len ok");
        self.max_len
    }

    fn recompute_max_name_len(&mut self) {
        debug!("recompute_max_name_len start");
        self.max_len = self
            .entries
            .keys()
            .map(|k| k.chars().count())
            .max()
            .unwrap_or(0);
        info!(max_len = self.max_len, "recompute_max_name_len ok");
    }
}

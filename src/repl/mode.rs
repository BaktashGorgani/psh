use std::sync::{Arc, RwLock};
use tracing::{debug, info};

#[derive(Clone)]
pub struct ModeState {
    current: Arc<RwLock<Option<String>>>,
    default: Arc<RwLock<Option<String>>>,
}

impl Default for ModeState {
    fn default() -> Self {
        Self::new()
    }
}

impl ModeState {
    pub fn new() -> Self {
        debug!("mode_state_new start");
        let s = Self {
            current: Arc::new(RwLock::new(None)),
            default: Arc::new(RwLock::new(None)),
        };
        info!("mode_state_new ok");
        s
    }

    pub fn get_current(&self) -> Option<String> {
        debug!("mode_state_get_current start");
        let v = self.current.read().ok().and_then(|g| (*g).clone());
        let present = v.is_some();
        info!(present = present, "mode_state_get_current ok");
        v
    }

    pub fn set_current(&self, name: Option<String>) {
        debug!(present = name.is_some(), "mode_state_set_current start");
        if let Ok(mut w) = self.current.write() {
            *w = name;
        }
        info!("mode_state_set_current ok");
    }

    pub fn get_default(&self) -> Option<String> {
        debug!("mode_state_get_default_target start");
        let v = self.default.read().ok().and_then(|g| (*g).clone());
        let present = v.is_some();
        info!(present = present, "mode_state_get_default_target ok");
        v
    }

    pub fn set_default(&self, name: Option<String>) {
        debug!(
            present = name.is_some(),
            "mode_state_set_default_target start"
        );
        if let Ok(mut w) = self.default.write() {
            *w = name;
        }
        info!("mode_state_set_default_target ok");
    }
}

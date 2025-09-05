use reedline::{
    self, Reedline, ReedlineEvent, default_emacs_keybindings,
    default_vi_insert_keybindings, default_vi_normal_keybindings,
};
use tracing::{debug, info};

use crate::runtime::{ReplSettings, config};

pub const MENU_SENTINEL: &str = "__PSH_MENU__";

pub fn make_reedline(settings: &ReplSettings) -> Reedline {
    debug!("make_reedline start");
    match settings.edit_menu {
        config::EditMode::Emacs => {
            let mut kb = default_emacs_keybindings();
            kb.add_binding(
                settings.menu_key.1,
                settings.menu_key.0,
                ReedlineEvent::ExecuteHostCommand(MENU_SENTINEL.into()),
            );
            let edit_mode = Box::new(reedline::Emacs::new(kb));
            let rl = Reedline::create().with_edit_mode(edit_mode);
            info!("make_reedline emacs ok");
            rl
        }
        config::EditMode::Vi => {
            let mut insert = default_vi_insert_keybindings();
            let mut normal = default_vi_normal_keybindings();
            insert.add_binding(
                settings.menu_key.1,
                settings.menu_key.0,
                ReedlineEvent::ExecuteHostCommand(MENU_SENTINEL.into()),
            );
            normal.add_binding(
                settings.menu_key.1,
                settings.menu_key.0,
                ReedlineEvent::ExecuteHostCommand(MENU_SENTINEL.into()),
            );
            let edit_mode = Box::new(reedline::Vi::new(insert, normal));
            let rl = Reedline::create().with_edit_mode(edit_mode);
            info!("make_reedline emacs ok");
            rl
        }
    }
}

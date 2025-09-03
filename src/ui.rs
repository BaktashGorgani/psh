use std::io::{Write, stdout};

use tracing::{debug, info};

use crate::error::{Result, UiError};

pub mod editor;
//pub mod prefix_menu;

pub use editor::prompt::PshPrompt;

pub fn ui_print(msg: &str) -> Result<()> {
    debug!(len = msg.len(), "ui_print start");
    stdout()
        .write_all(msg.as_bytes())
        .map_err(UiError::IoWrite)?;
    info!("ui_print ok");
    Ok(())
}

pub fn ui_println(msg: &str) -> Result<()> {
    debug!(len = msg.len(), "ui_print start");
    let mut out = stdout();
    out.write_all(b"\r").map_err(UiError::IoWrite)?;
    out.write_all(msg.as_bytes()).map_err(UiError::IoWrite)?;
    out.write_all(b"\r\n").map_err(UiError::IoWrite)?;
    out.flush().map_err(UiError::IoWrite)?;
    info!("ui_print ok");
    Ok(())
}

pub fn ui_flush() -> Result<()> {
    debug!("ui_flush start");
    stdout().flush().map_err(UiError::IoWrite)?;
    info!("ui_flush ok");
    Ok(())
}

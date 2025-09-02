use tracing::{debug, info};

use crate::{
    builtins::BuiltinContext,
    error::{BuiltinError, Result},
};

pub async fn handle(_ctx: &mut dyn BuiltinContext, args: &str) -> Result<()> {
    debug!(args = args, "builtin_quit_handle start");
    info!("quit requested");
    Err(BuiltinError::ExitRequested.into())
}

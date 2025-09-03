use anyhow::Result;
use clap::Parser;
use tracing::debug;

use psh::{repl, runtime};

mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();

    debug!("main start");

    let app = runtime::bootstrap(args.cols, args.rows, args.verbose).await?;
    let mut router = app.router;
    let settings = app.repl_settings;

    repl::run_line(&mut router, &settings).await?;

    Ok(())
}

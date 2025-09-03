use std::{fs, path::PathBuf};

use tracing::{debug, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{EnvFilter, Registry, fmt, layer::Layer, prelude::*, reload};

type FmtLayer = Box<dyn Layer<Registry> + Send + Sync>;

const LOG_DIR: &str = "logs";
const LOG_FILE: &str = "app.log";
const VERBOSITY_INFO: u8 = 0;
const VERBOSITY_DEBUG: u8 = 1;

pub struct LogControl {
    pub guards: Vec<WorkerGuard>,
    fmt_handle: reload::Handle<FmtLayer, Registry>,
}

pub fn init_logging_early(verbosity: u8) -> LogControl {
    debug!("init_logging_early start");
    let _ = fs::create_dir_all(LOG_DIR);

    let (writer, guard) = non_blocking(rolling::daily(LOG_DIR, LOG_FILE));
    let filter = match verbosity {
        VERBOSITY_INFO => {
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        }
        VERBOSITY_DEBUG => EnvFilter::new("debug"),
        _ => EnvFilter::new("trace"),
    };

    let fmt_layer = fmt::layer().with_writer(writer).with_ansi(false).boxed();
    let (fmt_layer, fmt_handle) = reload::Layer::new(fmt_layer);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(filter)
        .init();

    info!("init_logging_early ok");

    LogControl {
        guards: vec![guard],
        fmt_handle,
    }
}

pub fn reconfigure_logging_path(control: &mut LogControl, file_path: Option<PathBuf>) {
    debug!("reconfigure_logging_path start");
    let (writer, guard) = match file_path {
        Some(p) => {
            if let Some(dir) = p.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&p)
                .unwrap_or_else(|_| fs::File::create(&p).expect("create log file"));
            non_blocking(file)
        }
        None => non_blocking(rolling::daily(LOG_DIR, LOG_FILE)),
    };

    let new_fmt: FmtLayer = fmt::layer().with_writer(writer).with_ansi(false).boxed();
    let _ = control.fmt_handle.reload(new_fmt);
    control.guards = vec![guard];

    info!("reconfigure_logging_path ok");
}

pub fn set_verbosity(control: &mut LogControl, verbosity: u8) {
    debug!("set_verbosity start");
    let filter = match verbosity {
        VERBOSITY_INFO => {
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        }
        VERBOSITY_DEBUG => EnvFilter::new("debug"),
        _ => EnvFilter::new("trace"),
    }
    .boxed();
    let _ = control.fmt_handle.reload(filter);
    info!("set_verbosity ok");
}

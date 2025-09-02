use std::io::Error as IoError;

use anyhow::Error as AnyError;
use toml::de::Error as TomlError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("failed to read config at {path}")]
    ConfigRead {
        path: String,
        #[source]
        source: IoError,
    },

    #[error("failed to parse config at {path}")]
    ConfigParse {
        path: String,
        #[source]
        source: TomlError,
    },

    #[error("failed to reconfigure logging")]
    LoggingReconfigure {
        #[source]
        source: AnyError,
    },
}

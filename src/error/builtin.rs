use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuiltinError {
    #[error("unrecognized local command: {args}")]
    LocalUnrecognized { args: String },

    #[error("unrecognized remote command: {args}")]
    RemoteUnrecognized { args: String },

    #[error("unrecognized admin command: {args}")]
    AdminUnrecognized { args: String },

    #[error("invalid arguments: {detail}")]
    InvalidArgs { detail: String },

    #[error("user requested exit")]
    ExitRequested,
}

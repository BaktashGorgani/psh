#[derive(Clone, Debug)]
pub enum ShellEvent {
    Output(String),
    Exited(String),
}

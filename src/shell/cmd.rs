#[derive(Debug, Clone)]
pub enum ShellCmd {
    WriteLine(String),
    WriteBytes(Vec<u8>),
    Resize(u16, u16),
    Shutdown,
}

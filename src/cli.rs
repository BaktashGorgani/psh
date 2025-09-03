use clap::{ArgAction, Parser};

const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;

#[derive(Parser, Debug)]
#[command(name = "psh")]
#[command(version, about = "Parent Shell")]
pub struct Cli {
    #[arg(short, long, default_value_t = DEFAULT_COLS)]
    pub cols: u16,
    #[arg(short, long, default_value_t = DEFAULT_ROWS)]
    pub rows: u16,
    #[arg(short, long, default_value_t = true)]
    pub bash: bool,
    #[arg(short, long, default_value_t = true)]
    pub zsh: bool,
    #[arg(
        short,
        long,
        default_value_t = true,
        help = "Attach interactively to shell; double Ctrl-] to detach"
    )]
    pub interactive: bool,
    #[arg(short, long, action = ArgAction::Count)]
    pub verbose: u8,
}

use clap::{ArgAction, Parser};

#[derive(Parser, Debug)]
#[command(name = "psh")]
#[command(version, about = "Parent Shell")]
pub struct Cli {
    #[arg(short, long, default_value_t = 80)]
    pub cols: u16,
    #[arg(short, long, default_value_t = 24)]
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

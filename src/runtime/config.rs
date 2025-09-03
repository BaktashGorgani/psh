use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use crossterm::event::{KeyCode, KeyModifiers};
use directories::BaseDirs;
use nu_ansi_term::Color;
use serde::Deserialize;
use tracing::{debug, info, warn};
use users::{self, os::unix::UserExt};

use crate::shell::ShellSpec;

const DEFAULT_MENU_KEY: (KeyCode, KeyModifiers) =
    (KeyCode::Char('g'), KeyModifiers::CONTROL);
const MAX_FUNCTION_KEY: u8 = 24;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct PshConfig {
    pub logging: Option<LoggingSection>,
    pub shells: Option<ShellsSection>,
    pub repl: Option<ReplSection>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct LoggingSection {
    pub file: Option<String>,
    pub level: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ShellsSection {
    pub default_shell: Option<String>,
    pub catalog: Option<HashMap<String, ShellSpec>>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ReplSection {
    pub menu_key: Option<String>,
    pub colors: Option<ReplColors>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ReplColors {
    pub prompt: Option<String>,
    pub builtin: Option<String>,
    pub local: Option<String>,
    pub remote: Option<String>,
    pub unknown: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReplSettings {
    pub menu_key: (KeyCode, KeyModifiers),
    pub color_prompt: Color,
    pub color_builtin: Color,
    pub color_local: Color,
    pub color_remote: Color,
    pub color_unknown: Color,
}

fn parse_color(name: &str) -> Option<Color> {
    match name.to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "lightgrey" | "lightgray" => Some(Color::LightGray),
        "darkgrey" | "darkgray" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "red" => Some(Color::Red),
        "lightgreen" => Some(Color::LightGreen),
        "green" => Some(Color::Green),
        "lightyellow" => Some(Color::LightYellow),
        "yellow" => Some(Color::Yellow),
        "lightblue" => Some(Color::LightBlue),
        "blue" => Some(Color::Blue),
        "lightmagenta" => Some(Color::LightMagenta),
        "magenta" => Some(Color::Magenta),
        "lightcyan" => Some(Color::LightCyan),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        _ => None,
    }
}

fn parse_key_code(token: &str) -> Option<KeyCode> {
    let t = token.trim().to_ascii_lowercase();
    if t.chars().count() == 1 {
        match t.chars().next() {
            Some(ch) => return Some(KeyCode::Char(ch)),
            None => return None,
        }
    }
    match t.as_str() {
        "space" => Some(KeyCode::Char(' ')),
        "tab" => Some(KeyCode::Tab),
        "enter" | "return" => Some(KeyCode::Enter),
        "esc" | "escape" => Some(KeyCode::Esc),
        "backspace" | "bs" => Some(KeyCode::Backspace),
        "delete" | "del" => Some(KeyCode::Delete),
        "insert" | "ins" => Some(KeyCode::Insert),
        "pageup" | "pgup" => Some(KeyCode::PageUp),
        "pagedown" | "pgdn" => Some(KeyCode::PageDown),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        _ => match t.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
            Some(num) if (1..=MAX_FUNCTION_KEY).contains(&num) => Some(KeyCode::F(num)),
            _ => None,
        },
    }
}

fn parse_menu_key(s: &str) -> Option<(KeyCode, KeyModifiers)> {
    let mut mods = KeyModifiers::empty();
    let mut code: Option<KeyCode> = None;

    for raw in s.split(['+', '-']) {
        let tok = raw.trim().to_ascii_lowercase();
        if tok.is_empty() {
            continue;
        }
        let was_mod = match tok.as_str() {
            "super" | "win" | "windows" => {
                mods |= KeyModifiers::SUPER;
                true
            }
            "ctrl" | "control" => {
                mods |= KeyModifiers::CONTROL;
                true
            }
            "alt" | "option" => {
                mods |= KeyModifiers::ALT;
                true
            }
            "meta" => {
                mods |= KeyModifiers::META;
                true
            }
            "shift" => {
                mods |= KeyModifiers::SHIFT;
                true
            }
            _ => false,
        };
        if was_mod {
            continue;
        }
        if code.is_some() {
            return None;
        }
        code = parse_key_code(&tok);
        code?;
    }

    code.map(|c| (c, mods))
}

pub fn repl_settings_from_config(cfg: &PshConfig) -> ReplSettings {
    debug!("repl_settings_from_config start");
    let repl = cfg.repl.clone().unwrap_or_default();

    let menu_key = repl
        .menu_key
        .as_deref()
        .and_then(parse_menu_key)
        .unwrap_or(DEFAULT_MENU_KEY);

    let defaults = ReplColors {
        prompt: Some("White".into()),
        builtin: Some("Yellow".into()),
        local: Some("Green".into()),
        remote: Some("Blue".into()),
        unknown: Some("Red".into()),
    };
    let colors = repl.colors.unwrap_or(defaults);

    let color_prompt = colors
        .prompt
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(Color::White);
    let color_builtin = colors
        .builtin
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(Color::Yellow);
    let color_local = colors
        .local
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(Color::Green);
    let color_remote = colors
        .remote
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(Color::Blue);
    let color_unknown = colors
        .unknown
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(Color::Red);

    info!("repl_settings_from_config ok");
    ReplSettings {
        menu_key,
        color_prompt,
        color_builtin,
        color_local,
        color_remote,
        color_unknown,
    }
}

pub fn login_shell_program_name() -> Option<String> {
    debug!("login_shell_program_name start");
    if let Ok(shell_path) = env::var("SHELL")
        && let Some(name) = Path::new(&shell_path).file_name().and_then(|s| s.to_str())
    {
        info!(program = name, "login_shell_program_name enf ok");
        return Some(name.to_string());
    }

    if let Some(user) = users::get_user_by_uid(users::get_current_uid()) {
        let shell_path = user.shell().to_string_lossy().to_string();
        if let Some(name) = Path::new(&shell_path).file_name().and_then(|s| s.to_str())
        {
            info!(program = name, "login_shell_program_name passwd ok");
            return Some(name.to_string());
        }
    }
    warn!("login_shell_program_name unknown");
    None
}

pub fn load_config() -> (PshConfig, PathBuf) {
    debug!("load_config start");
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(s) => match toml::from_str::<PshConfig>(&s) {
            Ok(cfg) => {
                info!(path = %path.display(), "load_config ok");
                (cfg, path)
            }
            Err(e) => {
                warn!(path = %path.display(), ?e, "load_config parse failed");
                (PshConfig::default(), path)
            }
        },
        Err(e) => {
            warn!(path = %path.display(), ?e, "load_config read failed_using_defaults");
            (PshConfig::default(), path)
        }
    }
}

fn config_path() -> PathBuf {
    debug!("config_path start");
    let path = if let Ok(p) = env::var("PSH_CONFIG") {
        PathBuf::from(p)
    } else if let Some(base) = BaseDirs::new() {
        base.home_dir().join(".psh").join("config.toml")
    } else if let Ok(home) = env::var("HOME") {
        Path::new(&home).join(".psh").join("config.toml")
    } else {
        PathBuf::from(".psh/config.toml")
    };
    info!(path = %path.display(), "config_path ok");
    path
}

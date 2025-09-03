use nu_ansi_term::{Color, Style};
use reedline::{Highlighter, StyledText};
use tracing::{debug, info, warn};

use crate::{
    registry::{self, Registry},
    repl::parser::{self, Parsed},
    runtime::ReplSettings,
    shell::ShellSpec,
};

#[derive(Clone)]
pub struct PshHighlighter {
    registry: Registry,
    color_builtin: Color,
    color_local: Color,
    color_remote: Color,
    color_unknown: Color,
}

impl PshHighlighter {
    pub fn new(registry: Registry, settings: &ReplSettings) -> Self {
        debug!("psh_highlighter_new start");
        let s = Self {
            registry,
            color_builtin: settings.color_builtin,
            color_local: settings.color_local,
            color_remote: settings.color_remote,
            color_unknown: settings.color_unknown,
        };
        info!("psh_highlighter_new ok");
        s
    }

    pub fn update_registry(&mut self, registry: Registry) {
        debug!("psh_highlighter_update_registry start");
        self.registry = registry;
        info!("psh_highlighter_update_registry ok");
    }
}

impl Highlighter for PshHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        debug!(len = line.len(), "psh_highlight start");

        match line.find(':') {
            Some(colon_idx) => {
                let prefix = &line[..colon_idx];
                let rest = &line[colon_idx + 1..];

                let parsed = parser::parse(&self.registry, line);

                let color = match parsed {
                    Parsed::Entry {
                        ref name,
                        ref entry,
                        ..
                    } if name == prefix => match entry {
                        registry::Entry::Builtin => self.color_builtin,
                        registry::Entry::Shell(ShellSpec::Local { .. }) => {
                            self.color_local
                        }
                        registry::Entry::Shell(ShellSpec::Remote { .. }) => {
                            self.color_remote
                        }
                    },
                    _ => {
                        warn!(prefix = prefix, "psh_highlight prefix unknown");
                        self.color_unknown
                    }
                };

                let mut out: StyledText = StyledText { buffer: Vec::new() };
                out.push((Style::new().fg(color), prefix.to_string()));
                out.push((Style::new(), ":".to_string()));
                out.push((Style::new(), rest.to_string()));
                info!("psh_highlight ok");
                out
            }
            None => {
                let mut out: StyledText = StyledText { buffer: Vec::new() };
                out.push((Style::new(), line.to_string()));
                info!("psh_highlight ok");
                out
            }
        }
    }
}

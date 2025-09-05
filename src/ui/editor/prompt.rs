use std::borrow::Cow;

use nu_ansi_term::{Color, Style};
use reedline::{Prompt, PromptEditMode, PromptHistorySearch};
use tracing::{debug, info};

use crate::{
    registry::{self, Registry},
    repl::ModeState,
    runtime::ReplSettings,
    shell::ShellSpec,
};

const ANSI_RESET: &str = "\x1b[0m";

#[derive(Clone)]
pub struct PshPrompt {
    mode: Option<ModeState>,
    registry: Option<Registry>,
    color_prompt: Color,
    color_builtin: Color,
    color_local: Color,
    color_remote: Color,
    color_unknown: Color,
}

impl PshPrompt {
    pub fn new(settings: &ReplSettings) -> Self {
        debug!("psh_prompt_new start");
        let s = Self {
            mode: None,
            registry: None,
            color_prompt: settings.color_prompt,
            color_builtin: settings.color_builtin,
            color_local: settings.color_local,
            color_remote: settings.color_remote,
            color_unknown: settings.color_unknown,
        };
        info!("psh_prompt_new ok");
        s
    }

    pub fn set_mode_state(&mut self, mode: ModeState) {
        debug!("psh_prompt_set_mode_state start");
        self.mode = Some(mode);
        info!("psh_prompt_set_mode_state ok");
    }

    pub fn set_registry(&mut self, reg: Registry) {
        debug!("psh_prompt_set_registry start");
        self.registry = Some(reg);
        info!("psh_prompt_set_registry ok");
    }

    fn color_for_mode(&self, name: &str) -> Color {
        match &self.registry {
            Some(reg) => match reg.get_entry(name) {
                Some(registry::Entry::Builtin) => self.color_builtin,
                Some(registry::Entry::Shell(ShellSpec::Local { .. })) => {
                    self.color_local
                }
                Some(registry::Entry::Shell(ShellSpec::Remote { .. })) => {
                    self.color_remote
                }
                None => self.color_unknown,
            },
            None => self.color_unknown,
        }
    }
}

impl Prompt for PshPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        debug!("psh_prompt_render_left start");

        let prefix = Style::new()
            .fg(self.color_prompt)
            .paint("psh> ")
            .to_string();

        let current = self.mode.as_ref().and_then(|m| m.get_current());

        let suffix = match current {
            Some(name) => {
                let color = self.color_for_mode(&name);
                Style::new()
                    .fg(color)
                    .paint(format!("{name}: "))
                    .to_string()
            }
            None => String::new(),
        };

        let out = format!("{ANSI_RESET}{prefix}{suffix}");
        debug!("psh_prompt_render_left ok");
        Cow::Owned(out)
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("... ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        _history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Borrowed("(search) ")
    }
}

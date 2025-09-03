use std::borrow::Cow;

use nu_ansi_term::{Color, Style};
use reedline::{Prompt, PromptEditMode, PromptHistorySearch};
use tracing::{debug, info};

use crate::{
    registry::{self, Registry},
    runtime::ReplSettings,
    shell::ShellSpec,
};

const ANSI_RESET: &str = "\x1b[0m";

#[derive(Clone)]
pub struct PshPrompt {
    locked_prefix: Option<String>,
    color_prompt: Color,
    color_builtin: Color,
    color_local: Color,
    color_remote: Color,
    color_unknown: Color,
    registry: Option<Registry>,
}

impl PshPrompt {
    pub fn new(locked_prefix: Option<String>, settings: &ReplSettings) -> Self {
        debug!(present = locked_prefix.is_some(), "psh_prompt_new start");
        let s = Self {
            locked_prefix,
            color_prompt: settings.color_prompt,
            color_builtin: settings.color_builtin,
            color_local: settings.color_local,
            color_remote: settings.color_remote,
            color_unknown: settings.color_unknown,
            registry: None,
        };
        info!("psh_prompt_new ok");
        s
    }

    pub fn set_locked_prefix(&mut self, new_prefix: Option<String>) {
        debug!(
            present = new_prefix.is_some(),
            "psh_prompt_set_locked_prefix start"
        );
        self.locked_prefix = new_prefix;
        info!("psh_prompt_set_locked_prefix ok");
    }

    pub fn set_registry(&mut self, reg: Registry) {
        debug!("psh_prompt_set_registry start");
        self.registry = Some(reg);
        info!("psh_prompt_set_registry ok");
    }

    fn color_for_prefix(&self, name: &str) -> Color {
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

        let psh_colored = Style::new()
            .fg(self.color_prompt)
            .paint("psh> ")
            .to_string();

        let prefix_colored = match &self.locked_prefix {
            Some(name) => {
                let color = self.color_for_prefix(name);
                Style::new()
                    .fg(color)
                    .paint(format!("{name}: "))
                    .to_string()
            }
            None => String::new(),
        };

        let out = format!("{ANSI_RESET}{psh_colored}{prefix_colored}");
        debug!("psh_prompt_render_left ok");
        Cow::Owned(out)
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        debug!("psh_prompt_render_right start");
        debug!("psh_prompt_render_right ok");
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        debug!("psh_prompt_render_indicator start");
        debug!("psh_prompt_render_indicator ok");
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        debug!("psh_prompt_render_multiline start");
        debug!("psh_prompt_render_multiline ok");
        Cow::Borrowed("... ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        _history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        debug!("psh_prompt_render_history_search start");
        debug!("psh_prompt_render_history_search ok");
        Cow::Borrowed("(search) ")
    }
}

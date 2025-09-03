use tracing::{debug, info, warn};

use crate::{
    builtins::BuiltinContext,
    error::Result,
    registry::{self, Registry},
    repl::Router,
    runtime::{
        config::{self, PshConfig, ReplSettings, ShellsSection},
        logging::{LogControl, init_logging_early, reconfigure_logging_path},
    },
    shell::ShellSpec,
};

const DEFAULT_SHELL_NAME: &str = "bash";
const DEFAULT_SHELL_PATH: &str = "/usr/bin/bash";

pub struct AppParts {
    pub cfg: PshConfig,
    pub router: Router,
    pub default_shell: String,
    pub log_control: LogControl,
    pub repl_settings: ReplSettings,
}

fn build_base_registry() -> Registry {
    debug!("build_base_registry start");
    let r = Registry::with_builtins();
    info!("build_base_registry ok");
    r
}

fn apply_shells_from_config(cfg: &PshConfig, router: &mut Router) {
    debug!("apply_shells_from_config start");
    if let Some(ShellsSection { catalog, .. }) = &cfg.shells
        && let Some(map) = catalog
    {
        map.iter().for_each(|(name, spec)| {
                router.register_entry(name.clone(), registry::Entry::Shell(spec.clone()));
                info!(name = %name, shell = format!("{:?}", spec), "apply_shells_from_config entry");
            });
    }
}

async fn eager_start_registered_shells(router: &mut Router) {
    debug!("eager_start_registered_shells start");
    for (name, entry) in router.list_entries() {
        if let registry::Entry::Shell(_) = entry {
            match router.ensure_shell_session_by_name(&name).await {
                Ok(_) => info!(name = %name, "eager_start ok"),
                Err(e) => warn!(name = %name, ?e, "eager_start failed"),
            }
        }
    }
}

async fn ensure_fallback_bash(router: &mut Router) {
    debug!("ensure_fallback_bash start");
    let bash_present = router.list_entries().iter().any(|(name, entry)| {
        matches!(entry, registry::Entry::Shell(ShellSpec::Local { .. }) if name == DEFAULT_SHELL_NAME)
    });
    if !bash_present {
        router.register_entry(
            DEFAULT_SHELL_NAME.to_string(),
            registry::Entry::Shell(ShellSpec::Local {
                program: DEFAULT_SHELL_PATH.to_string(),
            }),
        );
        info!("fallback bash registered");
    }
    match router
        .ensure_shell_session_by_name(DEFAULT_SHELL_NAME)
        .await
    {
        Ok(_) => info!("fallback bash started"),
        Err(e) => warn!(?e, "fallback bash start failed "),
    }
    info!("ensure_fallback_bash ok");
}

pub async fn bootstrap(cols: u16, rows: u16, verbosity: u8) -> Result<AppParts> {
    debug!("bootstrap start");

    let mut log_control = init_logging_early(verbosity);

    let (cfg, cfg_path) = config::load_config();
    info!(config = %cfg_path.display(), "config loaded");

    let log_path = cfg
        .logging
        .as_ref()
        .and_then(|l| l.file.as_ref())
        .map(|s| s.into());
    reconfigure_logging_path(&mut log_control, log_path);

    let registry = build_base_registry();
    let mut router = Router::new(registry, None, cols, rows);
    info!("router initialized");

    apply_shells_from_config(&cfg, &mut router);
    eager_start_registered_shells(&mut router).await;
    ensure_fallback_bash(&mut router).await;

    let repl_settings = config::repl_settings_from_config(&cfg);

    let default_shell = cfg
        .shells
        .as_ref()
        .and_then(|s| s.default_shell.clone())
        .filter(|name| router.set_default_shell(name))
        .or_else(|| {
            config::login_shell_program_name().filter(|n| router.set_default_shell(n))
        })
        .unwrap_or_else(|| {
            if router.set_default_shell(DEFAULT_SHELL_NAME) {
                DEFAULT_SHELL_NAME.to_string()
            } else {
                warn!("fallback default shell set failed; using literal name");
                DEFAULT_SHELL_NAME.to_string()
            }
        });
    info!(default = %default_shell, "default shell chosen");

    info!("bootstrap ok");
    Ok(AppParts {
        cfg,
        router,
        default_shell,
        log_control,
        repl_settings,
    })
}

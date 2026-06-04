use std::sync::Arc;

#[cfg(not(test))]
use std::sync::OnceLock;

use rex_config::LoadedConfig;

#[cfg(not(test))]
static SETTINGS: OnceLock<Arc<LoadedConfig>> = OnceLock::new();

#[cfg(test)]
static SETTINGS: std::sync::Mutex<Option<Arc<LoadedConfig>>> = std::sync::Mutex::new(None);

pub fn init(config: Arc<LoadedConfig>) {
    #[cfg(not(test))]
    {
        let _ = SETTINGS.set(config);
    }
    #[cfg(test)]
    {
        *SETTINGS.lock().expect("settings lock") = Some(config);
    }
}

pub fn is_initialized() -> bool {
    #[cfg(not(test))]
    {
        SETTINGS.get().is_some()
    }
    #[cfg(test)]
    {
        SETTINGS.lock().expect("settings lock").is_some()
    }
}

pub fn get() -> Arc<LoadedConfig> {
    #[cfg(not(test))]
    {
        SETTINGS
            .get()
            .cloned()
            .unwrap_or_else(default_loaded_config)
    }
    #[cfg(test)]
    {
        SETTINGS
            .lock()
            .expect("settings lock")
            .clone()
            .unwrap_or_else(default_loaded_config)
    }
}

fn default_loaded_config() -> Arc<LoadedConfig> {
    #[cfg(test)]
    let effective = {
        let mut cfg = rex_config::RexConfig::defaults();
        cfg.workspace.allow_cwd_fallback = Some(true);
        cfg.sidecars.harness = Some("direct".to_string());
        cfg.sidecars.required = Some(false);
        if let Some(entry) = cfg.sidecars.list.first_mut() {
            entry.enabled = false;
        }
        cfg
    };
    #[cfg(not(test))]
    let effective = rex_config::RexConfig::defaults();
    Arc::new(LoadedConfig {
        rex_root: std::path::PathBuf::from("/tmp/rex-test"),
        global_path: None,
        project_path: None,
        effective,
    })
}

#[cfg(test)]
pub fn init_for_test(config: Arc<LoadedConfig>) {
    init(config);
}

#[cfg(test)]
pub fn reset_for_test() {
    *SETTINGS.lock().expect("settings lock") = None;
}

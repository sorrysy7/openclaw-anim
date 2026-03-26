use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_gateway_port")]
    pub gateway_port: u16,

    #[serde(default = "default_min_hold_ms")]
    pub min_hold_ms: u64,

    #[serde(default = "default_error_hold_ms")]
    pub error_hold_ms: u64,

    #[serde(default = "default_max_backoff_ms")]
    pub max_backoff_ms: u64,

    #[serde(default = "default_read_idle_timeout_ms")]
    pub read_idle_timeout_ms: u64,

    #[serde(default = "default_connect_timeout_ms")]
    pub connect_timeout_ms: u64,

    /// Window size (MVP). Later can add scale presets.
    #[serde(default = "default_window_width")]
    pub window_width: u32,

    #[serde(default = "default_window_height")]
    pub window_height: u32,

    /// UI: show debug status text overlay.
    #[serde(default = "default_show_status")]
    pub show_status: bool,

    /// UI: initial Spine animation key. If omitted, we fall back to spine_animations["idle"].
    #[serde(default)]
    pub initial_spine_animation: Option<String>,

    /// Optional: map internal actions -> Spine animation name keys.
    /// This lets you rename animations in Spine without changing code.
    #[serde(default)]
    pub spine_animations: std::collections::HashMap<String, String>,

    /// Optional: override mapping from Gateway tool name -> internal action id.
    /// This is an additive override layer; unknown tools still fall back to code defaults.
    #[serde(default)]
    pub tool_action_overrides: std::collections::HashMap<String, String>,
}

fn default_gateway_port() -> u16 {
    18789
}
fn default_min_hold_ms() -> u64 {
    300
}
fn default_error_hold_ms() -> u64 {
    1000
}
fn default_max_backoff_ms() -> u64 {
    30_000
}
fn default_read_idle_timeout_ms() -> u64 {
    60_000
}
fn default_connect_timeout_ms() -> u64 {
    3_000
}

fn default_window_width() -> u32 {
    320
}

fn default_window_height() -> u32 {
    320
}

fn default_show_status() -> bool {
    false
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut spine_animations = std::collections::HashMap::new();
        // Defaults assume Spine animation names match our internal action ids.
        for k in [
            "idle",
            "read",
            "write",
            "exec",
            "web_search",
            "web_fetch",
            "browser",
            "reply",
            "error",
        ] {
            spine_animations.insert(k.to_string(), k.to_string());
        }

        Self {
            gateway_port: default_gateway_port(),
            min_hold_ms: default_min_hold_ms(),
            error_hold_ms: default_error_hold_ms(),
            max_backoff_ms: default_max_backoff_ms(),
            read_idle_timeout_ms: default_read_idle_timeout_ms(),
            connect_timeout_ms: default_connect_timeout_ms(),
            window_width: default_window_width(),
            window_height: default_window_height(),
            show_status: default_show_status(),
            initial_spine_animation: None,
            spine_animations,
            tool_action_overrides: std::collections::HashMap::new(),
        }
    }
}

/// Load app config from (in priority order):
/// 1) `OPENCLAW_ANIM_CONFIG` file path (JSON)
/// 2) platform app config dir: `<appConfigDir>/config.json`
/// 3) defaults
///
/// Security: token is NOT part of this config.
pub fn load<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppConfig {
    // 1) explicit path
    if let Ok(p) = std::env::var("OPENCLAW_ANIM_CONFIG") {
        if let Ok(text) = std::fs::read_to_string(&p) {
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&text) {
                return cfg;
            }
        }
    }

    // 2) app config dir
    if let Ok(dir) = app.path().app_config_dir() {
        let path = dir.join("config.json");
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&text) {
                return cfg;
            }
        }
    }

    AppConfig::default()
}

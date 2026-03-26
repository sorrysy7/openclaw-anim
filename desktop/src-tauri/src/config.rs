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

    /// Spine camera X offset (world units). Default: 0.
    #[serde(default)]
    pub spine_cam_x: f64,

    /// Spine camera Y offset (world units). Default: 0.
    #[serde(default)]
    pub spine_cam_y: f64,

    /// Spine camera zoom (world units per canvas pixel). Default: auto (0 = auto-fit).
    #[serde(default)]
    pub spine_zoom: f64,

    /// Path to the Spine atlas file, relative to the app's public/spine/ directory.
    /// Default: "chibi-stickers.atlas"
    #[serde(default = "default_spine_atlas")]
    pub spine_atlas: String,

    /// Path to the Spine skeleton JSON file, relative to the app's public/spine/ directory.
    /// Default: "chibi-stickers.json"
    #[serde(default = "default_spine_json")]
    pub spine_json: String,

    /// Spine skin name to activate. Required for multi-skin assets (e.g. chibi-stickers).
    /// Run with show_status:true to see available skins in terminal logs.
    #[serde(default)]
    pub spine_skin: Option<String>,

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
fn default_spine_atlas() -> String {
    "chibi-stickers.atlas".to_string()
}
fn default_spine_json() -> String {
    "chibi-stickers.json".to_string()
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
            spine_atlas: default_spine_atlas(),
            spine_json: default_spine_json(),
            spine_skin: None,
            initial_spine_animation: None,
            spine_animations,
            tool_action_overrides: std::collections::HashMap::new(),
            spine_cam_x: 0.0,
            spine_cam_y: 0.0,
            spine_zoom: 0.0,
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
        println!("[config] looking for config at: {}", path.display());
        if let Ok(text) = std::fs::read_to_string(&path) {
            println!("[config] loaded from: {}", path.display());
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&text) {
                return cfg;
            } else {
                println!("[config] parse error, using defaults");
            }
        } else {
            // First run: auto-generate a default config so the user has something to edit
            println!("[config] not found, generating default config at: {}", path.display());
            if let Err(e) = std::fs::create_dir_all(&dir) {
                println!("[config] failed to create config dir: {e}");
            } else if let Err(e) = std::fs::write(&path, DEFAULT_CONFIG_TEMPLATE) {
                println!("[config] failed to write default config: {e}");
            } else {
                println!("[config] default config written — edit it and restart to apply");
            }
        }
    }

    AppConfig::default()
}

/// Default config written on first run.
/// Users can edit this file to customize the app; restart to apply changes.
const DEFAULT_CONFIG_TEMPLATE: &str = r#"{
  "window_width": 200,
  "window_height": 350,

  "spine_atlas": "chibi-stickers.atlas",
  "spine_json": "chibi-stickers.json",
  "spine_skin": "spineboy",
  "initial_spine_animation": "movement/idle-front",

  "spine_animations": {
    "idle":       "movement/idle-front",
    "read":       "emotes/thinking",
    "write":      "emotes/determined",
    "exec":       "emotes/excited",
    "web_search": "emotes/thinking",
    "web_fetch":  "emotes/thinking",
    "browser":    "emotes/thinking",
    "reply":      "emotes/wave",
    "error":      "emotes/scared"
  },

  "tool_action_overrides": {},

  "gateway_port": 18789,
  "show_status": false
}
"#;

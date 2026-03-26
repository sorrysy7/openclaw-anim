use std::time::Duration;

use tauri::{Emitter, Listener, Manager, WindowEvent};

mod config;
mod sse_client;
mod state_machine;

use config::load as load_config;
use sse_client::{run_sse_loop, SseClientConfig};
use state_machine::{Action, StateMachine, StateMachineConfig};

fn action_to_str(a: Action) -> &'static str {
    match a {
        Action::Idle => "idle",
        Action::Read => "read",
        Action::Write => "write",
        Action::Exec => "exec",
        Action::WebSearch => "web_search",
        Action::WebFetch => "web_fetch",
        Action::Browser => "browser",
        Action::Reply => "reply",
        Action::Error => "error",
    }
}

fn get_gateway_token() -> Option<String> {
    // Priority: env var
    if let Ok(v) = std::env::var("OPENCLAW_GATEWAY_TOKEN") {
        if !v.trim().is_empty() {
            return Some(v);
        }
    }

    // Fallback: ~/.openclaw/openclaw.json
    let home = std::env::var("HOME").ok()?;
    let path = format!("{}/.openclaw/openclaw.json", home);
    let text = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    json.get("gateway")?
        .get("auth")?
        .get("token")?
        .as_str()
        .map(|s| s.to_string())
}

#[tauri::command]
fn start_dragging(window: tauri::WebviewWindow) {
    let _ = window.start_dragging();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_dragging])
        .setup(|app| {
            let window = app.get_webview_window("main").expect("main window");

            // Load non-sensitive app config (token is NOT part of config)
            let app_cfg = load_config(&app.handle());
            let spine_map = app_cfg.spine_animations.clone();
            let show_status = app_cfg.show_status;
            let initial_spine_animation = app_cfg
                .initial_spine_animation
                .clone()
                .or_else(|| spine_map.get("idle").cloned());

            // Apply window style for desktop-pet MVP
            let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
                width: app_cfg.window_width as f64,
                height: app_cfg.window_height as f64,
            }));
            let _ = window.set_always_on_top(true);
            // Decorations/transparent are primarily configured in tauri.conf.json

            // Bridge frontend debug logs to Rust stdout (optional but helpful during dev)
            let _ = window.listen("anim://log", move |event: tauri::Event| {
                let payload = event.payload();
                if payload.is_empty() {
                    println!("[ui] (empty payload)");
                } else {
                    println!("[ui] {payload}");
                }
            });

            // Send UI config to frontend
            let _ = window.emit(
                "anim://config",
                serde_json::json!({
                  "showStatus": show_status,
                  "initialSpine": initial_spine_animation,
                }),
            );

            // Spawn background task: SSE → state machine → emit to UI
            let token = get_gateway_token().unwrap_or_default();

            // If token missing, we still run but will keep reconnecting; UI can show idle.
            let url = format!("http://127.0.0.1:{}/api/anim/events", app_cfg.gateway_port);

            let window_for_task = window.clone();

            tauri::async_runtime::spawn(async move {
                // State machine lives in Rust (frontend only receives sanitized action)
                let mut sm = StateMachine::new(StateMachineConfig {
                    min_hold: Duration::from_millis(app_cfg.min_hold_ms),
                    error_hold: Duration::from_millis(app_cfg.error_hold_ms),
                    tool_action_overrides: app_cfg.tool_action_overrides.clone(),
                });

                let cfg = SseClientConfig {
                    url,
                    bearer_token: token,
                    connect_timeout: Duration::from_millis(app_cfg.connect_timeout_ms),
                    read_idle_timeout: Duration::from_millis(app_cfg.read_idle_timeout_ms),
                    max_backoff: Duration::from_millis(app_cfg.max_backoff_ms),
                };

                // NOTE: Do not log token; do not forward raw data.
                let _ = run_sse_loop(cfg, move |ev| {
                    // Convert gateway event → UI event
                    if let Some(ui_ev) = sm.on_gateway_event(ev.ts, ev.run_id, &ev.phase, &ev.tool)
                    {
                        let action_id = action_to_str(ui_ev.action);
                        let spine_key = spine_map
                            .get(action_id)
                            .cloned()
                            .unwrap_or_else(|| action_id.to_string());

                        let payload = serde_json::json!({
                          "ts": ui_ev.ts_ms,
                          "action": action_id,
                          "spine": spine_key,
                          "phase": ui_ev.phase,
                        });
                        // Ignore emit errors (window closed)
                        let _ = window_for_task.emit("anim://event", payload);
                    }
                })
                .await;
            });

            // Basic: close app when window is closed
            let app_handle = app.handle().clone();
            window.on_window_event(move |e| {
                if matches!(e, WindowEvent::Destroyed) {
                    app_handle.exit(0);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

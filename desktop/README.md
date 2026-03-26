# openclaw-anim desktop

Desktop companion app (macOS + Windows) that renders a single character animation driven by OpenClaw tool-call events.

## Security-first architecture

- Desktop app connects to the **local** Gateway only: `127.0.0.1`.
- **Gateway token stays in the Rust backend**.
- Frontend receives only **sanitized internal events** (no params/results/paths/URLs/commands).

Data flow:

`Gateway SSE (/api/anim/events)` → `Rust SSE client` → `State machine` → `Tauri internal event` → `UI renderer (Spine)`

## Event source (Gateway SSE)

OpenClaw plugin exposes SSE:

- `GET http://127.0.0.1:<port>/api/anim/events`
- Header: `Authorization: Bearer <gateway token>`
- Response: `Content-Type: text/event-stream`

Minimal payloads (content-free):

```json
{ "ts": 0, "runId": "...", "phase": "start|end|error|reply_start|reply_end", "tool": "read|write|exec|..." }
```

## Frontend events (sanitized)

Rust emits to the WebView:

- event name: `anim://event`
- payload shape:

```json
{
  "ts": 0,
  "action": "idle|read|write|exec|web_search|web_fetch|browser|reply|error",
  "spine": "<animation-name-key>",
  "phase": "enter"
}
```

`spine` is computed from config `spine_animations[action]` (defaults to identity).

## Configuration (non-sensitive)

Config does **not** include the Gateway token.

Load order:

1) `OPENCLAW_ANIM_CONFIG` → absolute path to a JSON config file
2) Platform app config dir → `<appConfigDir>/config.json`
3) Defaults

Example `config.json`:

```json
{
  "gateway_port": 18789,
  "min_hold_ms": 300,
  "error_hold_ms": 1000,
  "max_backoff_ms": 30000,
  "read_idle_timeout_ms": 60000,
  "connect_timeout_ms": 3000,
  "window_width": 320,
  "window_height": 320,
  "show_status": false,
  "initial_spine_animation": "emotes/just-right",
  "spine_animations": {
    "idle": "idle",
    "read": "read",
    "write": "write",
    "exec": "exec",
    "web_search": "search",
    "web_fetch": "fetch",
    "browser": "browser",
    "reply": "talk",
    "error": "error"
  },
  "tool_action_overrides": {
    "feishu_doc": "write",
    "tts": "reply"
  }
}
```

### Token (never in config)

Token is loaded only from:

- `OPENCLAW_GATEWAY_TOKEN` (preferred)
- or `~/.openclaw/openclaw.json` (`gateway.auth.token`)

## Dev

```bash
cd desktop
npm install
source "$HOME/.cargo/env"
npm run tauri dev
```

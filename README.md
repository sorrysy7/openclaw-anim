# openclaw-anim

> **OpenClaw 工作状态可视化桌面应用**
> 实时展示 AI 工具调用的对应骨骼动画，让你清楚知道 Agent 在做什么。

---

## 项目概述

当 OpenClaw Agent 执行工具调用时，桌面浮窗中的卡通角色会播放对应骨骼动画：

| Agent 行为 | 角色动作 |
|---|---|
| `read / memory_get / memory_search` | 思考 / 翻阅 📖 |
| `write / edit` | 书写 ✍️ |
| `exec / process` | 操作终端 ⚙️ |
| `web_search / web_fetch` | 搜索 🔍 |
| `browser` | 浏览 🌐 |
| 消息发送中 | 挥手回复 📤 |
| 空闲 | 待机 😌 |
| 工具报错 | 惊吓 😱 |

动画映射完全可配置，支持任意 Spine 资源替换。

---

## 技术架构

```
[OpenClaw Agent]
    ↓  before_tool_call / after_tool_call / message_sending / message_sent
[openclaw-anim plugin]          ← OpenClaw Plugin SDK (TypeScript)
    ↓  { ts, runId, phase, tool }  仅元数据，无任何内容
    ↓  GET /api/anim/events (SSE, Bearer Token)
[Tauri 2 Desktop App]
    ↓  Rust backend 订阅 SSE，token 不下发前端
[FSM 状态机 (Rust)]             ← 去抖、优先级、最小持续时间
    ↓  { action, spine, phase }  纯净动画指令
[Spine WebGL 前端]              ← 骨骼动画渲染
```

---

## 快速开始

### 环境要求

| 工具 | 版本 |
|---|---|
| Node.js | ≥ 18 |
| Rust + Cargo | stable 或 nightly |
| MSVC Build Tools | Windows 必须（勾选"Desktop development with C++"） |
| WebView2 | Windows 已内置（Edge 86+）|

### 1. 部署插件

```bash
# 复制插件到 OpenClaw extensions 目录
# macOS / Linux
cp -r plugin ~/.openclaw/extensions/openclaw-anim

# Windows
xcopy /E /I plugin %USERPROFILE%\.openclaw\extensions\openclaw-anim
```

在 `~/.openclaw/openclaw.json` 中加入：

```json
{
  "plugins": {
    "allow": ["openclaw-anim"]
  }
}
```

重启 Gateway：

```bash
openclaw gateway restart
```

验证插件已加载：

```bash
openclaw status
# 应看到：[openclaw-anim] Plugin registered — SSE endpoint: GET /api/anim/events
```

### 2. 放置 Spine 资源

将你的 Spine 导出文件放入：

```
desktop/public/spine/
├── chibi-stickers.atlas
├── chibi-stickers.json
└── chibi-stickers.png   （或 atlas 中引用的图片）
```

> 文件名需与 `desktop/src/main.ts` 中的 `atlasUrl` / `jsonUrl` 一致，默认为 `chibi-stickers.*`。

推荐资源：[Spine chibi-stickers 示例](https://zh.esotericsoftware.com/spine-examples-chibi-stickers)

### 3. 配置

创建配置文件（首次运行后 Tauri 会打印实际路径）：

**macOS：** `~/Library/Application Support/ai.openclaw.animdesktop/config.json`
**Windows：** `%APPDATA%\ai.openclaw.animdesktop\config.json`

参考 `config.example.json` 填写，最简配置：

```json
{
  "window_width": 200,
  "window_height": 350,
  "spine_skin": "spineboy",
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
  }
}
```

### 4. 运行

```bash
cd desktop
npm install
npm run tauri dev
```

---

## 配置文件说明

完整参数见 [`config.example.json`](./config.example.json)，核心参数：

| 参数 | 默认值 | 说明 |
|---|---|---|
| `window_width` | 320 | 浮窗宽度（px） |
| `window_height` | 320 | 浮窗高度（px） |
| `spine_atlas` | `chibi-stickers.atlas` | Spine atlas 文件名（相对于 `public/spine/`） |
| `spine_json` | `chibi-stickers.json` | Spine JSON 文件名（相对于 `public/spine/`） |
| `spine_skin` | 无 | Spine 皮肤名，多皮肤资源必填 |
| `initial_spine_animation` | `spine_animations.idle` | 启动时播放的动画 |
| `spine_animations` | action 名直通 | action → Spine 动画名映射表 |
| `tool_action_overrides` | 空 | 自定义工具 → action 映射 |
| `gateway_port` | 18789 | OpenClaw Gateway 端口 |
| `show_status` | false | 调试用状态文字覆盖层 |

---

## 项目结构

```
openclaw-anim/
├── README.md                 # 本文件
├── config.example.json       # 配置文件参考（含注释）
├── plugin/                   # OpenClaw 插件
│   ├── index.ts              # 插件主体，捕获 hook 事件 → SSE 推送
│   ├── openclaw.plugin.json  # 插件元数据
│   └── package.json
└── desktop/                  # Tauri 桌面应用
    ├── src/
    │   ├── main.ts           # 前端入口，监听事件 → 驱动动画
    │   └── spine_renderer.ts # Spine WebGL 渲染器封装
    ├── src-tauri/
    │   └── src/
    │       ├── lib.rs         # 应用入口，初始化窗口和后台任务
    │       ├── config.rs      # 配置加载
    │       ├── sse_client.rs  # SSE 长连接客户端（自动重连）
    │       └── state_machine.rs # 动画状态机（去抖/优先级）
    └── public/
        └── spine/             # Spine 资源目录（不含于仓库）
```

---

## 隐私与安全

- **零内容传输**：事件只含 `{ ts, runId, phase, tool }`，无文件路径、无命令内容、无 API Key
- **Token 隔离**：Gateway Bearer Token 只存在于 Rust backend，不暴露给前端 WebView
- **SSE 鉴权**：`GET /api/anim/events` 需要有效的 Gateway Bearer Token

---

## 里程碑

| 阶段 | 目标 | 状态 |
|---|---|---|
| M0 | 项目创建 + 文档 + 架构设计 | ✅ 完成 |
| M1 | 插件：捕获 hook → SSE 推送 | ✅ 完成 |
| M2 | 桌面端：Rust SSE client + 状态机 | ✅ 完成 |
| M3 | Spine WebGL 渲染 + 皮肤/动画配置 | ✅ 完成 |
| M4 | 全平台（Mac + Windows）稳定运行 | ✅ 完成 |
| M5 | 打包分发 / 自定义 Spine 资源工作流 | ⏳ 待开始 |

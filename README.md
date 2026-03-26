# openclaw-anim

> **OpenClaw 工作状态可视化桌面应用**
> 实时展示 AI 工具调用的对应骨骼动画，让你清楚看到我在做什么。

---

## 项目概述

当 OpenClaw Agent（强尼）执行工具调用时，桌面浮窗中的卡通角色会播放对应骨骼动画：

| 工具操作 | 角色动作 |
|---|---|
| `read / memory_get / memory_search` | 翻书 / 扫视 📖 |
| `write / edit / feishu_doc` | 打字 / 书写 ✍️ |
| `exec / process` | 操控终端 ⚙️ |
| `web_search / web_fetch` | 放大镜搜索 🔍 |
| `browser` | 浏览 🌐 |
| `tts` | 说话口型 🗣️ |
| `message` | 发送消息 📤 |
| 空闲 / 输出 | 待机 / 说话 😌 |

---

## 技术架构

```
[OpenClaw Agent]
    ↓  before_tool_call / after_tool_call / message_sending / message_sent Plugin Hooks
[openclaw-anim-plugin（OpenClaw 插件）]
    ↓  结构化事件（仅工具类型 + 阶段，无内容/无敏感信息）
    ↓  api.registerHttpRoute → GET /api/anim/events (SSE)
[桌面应用（Tauri 2 + React）]
    ↓  Rust backend 订阅 SSE（token 不下发前端）
[FSM 状态机]
    ↓  idle / reading / writing / searching / executing / browsing / speaking / sending
[Spine WebGL 骨骼动画]
```

### 技术选型

| 模块 | 技术 | 选型理由 |
|---|---|---|
| OpenClaw 插件 | TypeScript + Plugin SDK | 原生支持，无需改 OpenClaw 源码 |
| 桌面框架 | **Tauri 2** | 安装包 ~5MB、内存 ~30MB、跨平台 Mac+Win |
| 前端渲染 | React + TypeScript | 生态完善，与 Tauri WebView 适配 |
| 动画引擎 | **Spine WebGL** | 支持骨骼动画，canvas 渲染，可扩展皮肤 |
| 事件传输 | SSE（Server-Sent Events） | 单向推送，轻量，浏览器原生支持 |

---

## 项目结构

```
openclaw-anim/
├── README.md               # 本文件
├── docs/
│   ├── requirements.md     # 需求规格
│   ├── architecture.md     # 架构设计详解
│   ├── event-protocol.md   # 事件协议规范
│   └── milestones.md       # 里程碑计划
├── plugin/                 # OpenClaw 插件（M1）
│   ├── package.json
│   ├── openclaw.plugin.json
│   └── index.ts
└── desktop/                # Tauri 桌面应用（M2+）
    ├── src-tauri/
    └── src/
```

---

## 里程碑

| 阶段 | 目标 | 状态 |
|---|---|---|
| **M0** | 项目创建 + 文档 + 类型签名调研 | ✅ 进行中 |
| **M1** | 插件：捕获 tool 事件 → SSE 推送 → 测试验证 | ✅ 已完成 |
| **M2** | 桌面端 A：Rust SSE client（重连+脱敏）→ 控制台打印内部事件 | ⏳ 待开始 |
| **M3** | Spine 动画：Idle + Reading + Writing 三个动作跑通 | ⏳ 待开始 |
| **M4** | 补全所有工具映射 + always-on-top + 窗口打磨 | ⏳ 待开始 |

---

## 隐私与安全

- 事件载荷**不含任何内容**（无路径全文、无命令原文、无 URL query、无 API Key）
- 只传：`{ ts, runId, phase, tool }` 四个字段
- SSE 端点受 Gateway 鉴权保护（Bearer Token）
- **开源安全策略：**token 只存在于桌面端 Rust backend；前端仅接收脱敏后的内部事件（不含 params/results/path/url/command）

### SSE 端点（实际）

- `GET http://127.0.0.1:18789/api/anim/events`
- `Content-Type: text/event-stream`
- Header: `Authorization: Bearer <gateway token>`

> 注意：不要用 `/anim/events`（会被 Control UI 的 SPA fallback 返回 HTML）。
- 敏感信息严禁外泄（关键规则第 3 条）

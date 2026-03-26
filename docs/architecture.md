# 架构设计（Architecture）

> 版本：v0.1 | 日期：2026-03-25

---

## 1. 整体数据流

```
┌─────────────────────────────────────────────────────┐
│                  OpenClaw Gateway                    │
│                                                      │
│  Agent Loop                                          │
│    ├── [before_tool_call hook] ──→ 插件捕获          │
│    │       tool: "read", phase: "start"              │
│    ├── [工具执行中...]                                │
│    └── [after_tool_call hook]  ──→ 插件捕获          │
│            tool: "read", phase: "end"                │
│                                                      │
│  openclaw-anim-plugin                                │
│    ├── 接收 hook 事件                                 │
│    ├── 脱敏：只保留 tool + phase + ts + runId        │
│    ├── 广播到所有 SSE 连接                            │
│    └── GET /api/anim/events  ←────────────────────┐      │
└───────────────────────────────────────────────┼──────┘
                                                │ SSE
┌───────────────────────────────────────────────┼──────┐
│              Tauri 桌面应用                    │      │
│                                               │      │
│  EventSource("http://localhost:port/api/anim/events")     │
│    └── onmessage → FSM.dispatch(event)        │      │
│                                                      │
│  FSM（有限状态机）                                    │
│    状态：idle / reading / writing / executing /      │
│           searching / browsing / speaking /          │
│           sending / thinking                         │
│    转换规则：                                         │
│      start → 切入对应状态                             │
│      end/error → 回到 idle（无其他并发时）             │
│      并发：取最高优先级状态                            │
│                                                      │
│  Spine WebGL 渲染层                                  │
│    ├── <canvas> 全屏                                 │
│    ├── 监听 FSM 状态变化                             │
│    └── crossFade(animName, 0.3s) 切换动画            │
└─────────────────────────────────────────────────────┘
```

---

## 2. 插件模块设计（M1）

### 文件结构
```
plugin/
├── package.json
├── openclaw.plugin.json
└── index.ts
```

### 核心逻辑（index.ts）

```typescript
// 伪代码，实际类型签名待 M1 调研确认
import { definePluginEntry } from "openclaw/plugin-sdk/plugin-entry";

// SSE 连接管理
const clients = new Set<ServerResponse>();

export default definePluginEntry({
  id: "openclaw-anim",
  name: "OpenClaw Anim",
  description: "Broadcasts tool call events via SSE for animation desktop app",

  register(api) {
    // 1. SSE 端点
    api.registerHttpRoute({
      path: "/api/anim/events",
      auth: "gateway",         // 使用 Gateway Bearer Token 鉴权
      match: "exact",
      handler: async (req, res) => {
        res.setHeader("Content-Type", "text/event-stream");
        res.setHeader("Cache-Control", "no-cache");
        res.setHeader("Connection", "keep-alive");
        res.statusCode = 200;
        clients.add(res);
        req.on("close", () => clients.delete(res));
        // 发送心跳防断连
        const heartbeat = setInterval(() => {
          res.write(": heartbeat\n\n");
        }, 30000);
        req.on("close", () => clearInterval(heartbeat));
        return true;
      },
    });

    // 2. 工具调用 Hook（待确认实际 API）
    api.registerHook(
      ["before_tool_call", "after_tool_call"],
      (event) => {
        const payload = JSON.stringify({
          ts: Date.now(),
          runId: event.runId ?? "unknown",
          phase: event.type === "before_tool_call" ? "start" : "end",
          tool: event.toolName,   // 只取工具名，不取参数/结果
        });
        const data = `data: ${payload}\n\n`;
        for (const client of clients) {
          client.write(data);
        }
      }
    );
  },
});
```

### 工具 → 状态映射表

| 工具名 | 动画状态 | 优先级 |
|---|---|---|
| `exec`, `process` | `executing` | 8 |
| `write`, `edit` | `writing` | 7 |
| `browser` | `browsing` | 6 |
| `web_search`, `web_fetch` | `searching` | 5 |
| `read`, `memory_search`, `memory_get` | `reading` | 4 |
| `message` | `sending` | 3 |
| `tts` | `speaking` | 2 |
| 其他 | `thinking` | 1 |
| 无工具 | `idle` | 0 |

---

## 3. 桌面应用模块设计（M2-M4）

### FSM 状态机（fsm.ts）

```typescript
type AnimState =
  | "idle" | "reading" | "writing" | "executing"
  | "searching" | "browsing" | "speaking" | "sending" | "thinking";

interface ToolEvent {
  ts: number;
  runId: string;
  phase: "start" | "end" | "error";
  tool: string;
}

class AnimFSM {
  // 当前活跃工具调用集合（支持并发）
  private activeTools = new Map<string, { tool: string; state: AnimState; priority: number }>();
  private currentState: AnimState = "idle";

  dispatch(event: ToolEvent): AnimState {
    const key = `${event.runId}:${event.tool}`;
    if (event.phase === "start") {
      this.activeTools.set(key, {
        tool: event.tool,
        state: toolToState(event.tool),
        priority: toolToPriority(event.tool),
      });
    } else {
      this.activeTools.delete(key);
    }
    // 取最高优先级
    this.currentState = this.resolveState();
    return this.currentState;
  }

  private resolveState(): AnimState {
    if (this.activeTools.size === 0) return "idle";
    let best = { priority: -1, state: "idle" as AnimState };
    for (const entry of this.activeTools.values()) {
      if (entry.priority > best.priority) best = entry;
    }
    return best.state;
  }
}
```

### Tauri 窗口配置

```json
// tauri.conf.json（关键配置）
{
  "windows": [{
    "alwaysOnTop": true,
    "decorations": false,
    "transparent": true,
    "width": 200,
    "height": 200,
    "resizable": false
  }]
}
```

---

## 4. 关键风险与缓解

| 风险 | 严重度 | 缓解方案 |
|---|---|---|
| `before_tool_call` hook 签名不符合预期 | 高 | M1 阶段优先翻源码 `.d.ts` 确认 |
| `registerHttpRoute` 不支持 SSE 长连接 | 高 | M1 实测；备选：轮询 HTTP |
| Spine WebGL 在 Tauri WebView 兼容性 | 中 | M3 验证；备选：Lottie |
| Gateway SSE 端口跨域问题 | 低 | Tauri 端设置正确 Origin 或 CORS |
| 并发工具调用状态跳变影响观感 | 低 | FSM 优先级规则 + 动画 crossFade 平滑过渡 |

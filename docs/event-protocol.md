# 事件协议规范（Event Protocol）

> 版本：v0.1 | 日期：2026-03-25

---

## SSE 端点

```
GET http://<gateway-host>:<port>/api/anim/events
Authorization: Bearer <gateway-token>
Accept: text/event-stream
```

## 事件格式

每条事件遵循 SSE 标准格式：

```
data: {"ts":1711370000000,"runId":"abc-123","phase":"start","tool":"read"}\n\n
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `ts` | `number` | Unix 毫秒时间戳 |
| `runId` | `string` | Agent 本次运行 ID（用于并发追踪） |
| `phase` | `"start" \| "end" \| "error"` | 工具调用阶段 |
| `tool` | `string` | 工具名称（如 `read`, `exec`） |

### 严格禁止包含

- ❌ 工具调用参数（文件路径、命令内容、URL、查询词等）
- ❌ 工具执行结果
- ❌ 任何密钥、Token、密码
- ❌ 用户文件内容

## 心跳

每 30 秒发送一次注释行防止连接超时断开：

```
: heartbeat\n\n
```

## 客户端处理示例

```typescript
const es = new EventSource("http://localhost:18789/api/anim/events", {
  headers: { Authorization: `Bearer ${token}` },
});

es.onmessage = (e) => {
  const event = JSON.parse(e.data);
  fsm.dispatch(event);
};

es.onerror = () => {
  // 断线重连（EventSource 默认自动重连）
  console.warn("[anim] SSE disconnected, reconnecting...");
};
```

## 工具 → 动画状态映射

```typescript
const TOOL_STATE_MAP: Record<string, AnimState> = {
  // Reading
  read: "reading",
  memory_get: "reading",
  memory_search: "reading",
  pdf: "reading",
  image: "reading",
  feishu_doc_read: "reading",

  // Writing
  write: "writing",
  edit: "writing",
  feishu_doc_write: "writing",
  feishu_doc_append: "writing",

  // Executing
  exec: "executing",
  process: "executing",

  // Searching
  web_search: "searching",
  web_fetch: "searching",

  // Browsing
  browser: "browsing",
  canvas: "browsing",

  // Speaking
  tts: "speaking",

  // Sending
  message: "sending",

  // Fallback
  _default: "thinking",
};

const PRIORITY: Record<AnimState, number> = {
  executing: 8,
  writing: 7,
  browsing: 6,
  searching: 5,
  reading: 4,
  sending: 3,
  speaking: 2,
  thinking: 1,
  idle: 0,
};
```

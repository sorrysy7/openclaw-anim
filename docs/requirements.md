# 需求规格（Requirements）

> 版本：v0.1 | 日期：2026-03-25 | 作者：强尼

---

## 1. 背景与目标

为 OpenClaw AI 助手（强尼）开发一个桌面浮窗应用，**实时可视化**其工作状态。
当 Agent 执行不同工具调用时，浮窗中的卡通角色播放对应的骨骼动画，让用户直观感知 AI 正在"做什么"。

## 2. 用户故事

- 作为用户，我希望看到一个桌面小窗口，里面有一个可爱的卡通角色
- 当 AI 在读文件时，角色做翻书动作
- 当 AI 在搜索网络时，角色做放大镜/思考动作
- 当 AI 在写文件时，角色做打字动作
- 当 AI 在执行命令时，角色做操作终端动作
- 当 AI 输出回复时，角色做说话动作
- 无操作时，角色保持待机动画

## 3. 功能需求（明确要求的）

### 3.1 工具事件监听
- 监听以下工具调用的开始（`start`）和结束（`end`/`error`）：
  - `read` → 状态：`reading`
  - `write`, `edit` → 状态：`writing`
  - `exec`, `process` → 状态：`executing`
  - `web_search`, `web_fetch` → 状态：`searching`
  - `browser` → 状态：`browsing`
  - `tts` → 状态：`speaking`
  - `message` → 状态：`sending`
  - `memory_search`, `memory_get` → 状态：`reading`
  - 其他工具 → 状态：`thinking`（通用兜底）
  - 无工具调用 → 状态：`idle`
  - 模型输出阶段 → 状态：`speaking`

### 3.2 动画展示
- 每个状态对应一个 Spine 骨骼动画（动画名与状态名一一对应）
- `start` 事件 → 切入对应状态动画（循环播放）
- `end`/`error` 事件 → 退回 `idle` 动画
- 并发工具调用时：按优先级取最高优先级状态（exec > writing > browsing > searching > reading > sending > speaking > thinking > idle）

### 3.3 事件传输协议
- OpenClaw 插件通过 SSE（Server-Sent Events）推送事件
- 端点：`GET /anim/events`，挂载在 OpenClaw Gateway HTTP 服务器
- 鉴权：使用 Gateway Bearer Token
- 事件载荷（严格不含敏感信息）：
  ```json
  {
    "ts": 1711370000000,
    "runId": "uuid-string",
    "phase": "start | end | error",
    "tool": "read | write | exec | ..."
  }
  ```

### 3.4 桌面应用
- 平台：macOS + Windows（双平台）
- 框架：Tauri 2
- 窗口形态：小浮窗，支持 always-on-top
- 动画引擎：Spine WebGL（canvas 渲染）
- 订阅方式：浏览器原生 `EventSource` API 连接 SSE 端点

## 4. 非功能需求

- **安装包大小**：目标 ≤ 20MB
- **内存占用**：目标 ≤ 60MB（Tauri 基准 ~30MB）
- **CPU 占用**：idle 状态下动画循环 CPU ≤ 5%
- **隐私**：事件载荷不含任何内容性信息，只含工具名称和执行阶段
- **安全**：SSE 端点需 Gateway 鉴权，密钥不得出现在任何网络传输或日志中

## 5. 明确不做的（Out of Scope）

- ❌ 不展示工具调用的具体内容（路径、命令、URL、结果等）
- ❌ 不做多角色/皮肤切换（M4 之前）
- ❌ 不做历史回放功能
- ❌ 不做性能面板展示
- ❌ 不修改 OpenClaw 核心源码

## 6. 依赖与假设

- OpenClaw Plugin SDK 的 `before_tool_call` / `after_tool_call` hooks 存在且可用（待 M1 实测验证）
- `api.registerHttpRoute` 支持 SSE 长连接（待验证）
- 用户已有 Gateway 访问凭证（配置桌面应用时需填入）

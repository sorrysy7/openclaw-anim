# 里程碑计划（Milestones）

> 版本：v0.1 | 日期：2026-03-25

---

## M0 — 项目准备（当前）✅

**目标：** 建立项目结构、文档、完成技术调研

**任务清单：**
- [x] 创建项目目录结构
- [x] 编写 README.md
- [x] 编写需求规格文档
- [x] 编写架构设计文档
- [x] 编写事件协议规范
- [x] 阅读 OpenClaw Plugin SDK 官方文档
- [ ] 翻阅源码 `.d.ts`，确认以下 API 的精确签名：
  - `api.registerHook` 的 hook 名称列表（`before_tool_call` 是否存在）
  - hook handler 的 event 参数结构（`toolName`、`runId` 等字段名）
  - `api.registerHttpRoute` 的 handler 参数类型（`req`/`res` 的具体类型）
  - SSE 长连接在 `registerHttpRoute` 中是否支持（`res.write` 是否可用）

---

## M1 — OpenClaw 插件：事件捕获 + SSE 推送 ⏳

**目标：** 插件能正确捕获工具调用事件并通过 SSE 推送，**curl 测试验证可行**

**前置条件：** M0 类型签名调研完成

**任务清单：**
- [ ] 确认 hook API 精确签名
- [ ] 创建 `plugin/package.json`
- [ ] 创建 `plugin/openclaw.plugin.json`
- [ ] 实现 `plugin/index.ts`：
  - [ ] `registerHttpRoute` → SSE 端点 `/api/anim/events`
  - [ ] `registerHook(["before_tool_call", "after_tool_call"], ...)` → 广播事件
  - [ ] 心跳机制（30s）
  - [ ] 连接管理（客户端断开时清理）
- [ ] 安装插件到 OpenClaw
- [ ] 重启 Gateway

**验证方案：**
```bash
# 终端 1：监听 SSE
curl -N -H "Authorization: Bearer <token>" \
  http://localhost:<port>/api/anim/events

# 终端 2：在对话中触发工具调用（如读一个文件）
# 预期：终端 1 看到类似输出
# data: {"ts":1711370000000,"runId":"xxx","phase":"start","tool":"read"}
# data: {"ts":1711370001200,"runId":"xxx","phase":"end","tool":"read"}
```

**通过标准：**
- [ ] `/api/anim/events` 端点可访问（返回 200 + `text/event-stream`）
- [ ] 每次工具调用触发 `start` 事件
- [ ] 工具完成后触发 `end` 事件
- [ ] 事件载荷无任何内容性信息（只有 ts/runId/phase/tool）
- [ ] 多个并发工具调用时，每个都有独立事件

**失败回退方案：**
- 若 `before_tool_call` hook 不存在 → 改用 `message:sent` hook 作为降级（只能检测到对话完成，无法细分工具）
- 若 `registerHttpRoute` 不支持 SSE → 改用轮询端点（每次 GET 返回当前状态快照）

---

## M2 — Tauri 桌面应用：SSE 订阅 + 状态打印 ⏳

**目标：** 桌面应用窗口能订阅 SSE，控制台打印状态变化

**前置条件：** M1 验证通过

**任务清单：**
- [ ] 初始化 Tauri 2 项目
- [ ] 实现 FSM 状态机（`src/fsm.ts`）
- [ ] 实现 SSE 订阅（`src/sse.ts`）
- [ ] 配置界面：填写 Gateway URL + Token
- [ ] 状态变化时在 DevTools 控制台打印

**验证方案：**
- 启动桌面应用，填入配置，触发对话
- DevTools 控制台看到状态变化日志

---

## M3 — Spine 动画：三个基础动作跑通 ⏳

**目标：** idle / reading / writing 三个骨骼动画正确播放并切换

**前置条件：** M2 验证通过

**任务清单：**
- [ ] 准备 Spine 动画资源（idle / reading / writing）
- [ ] 集成 spine-webgl 到 React
- [ ] 实现 `crossFade(animName, 0.3)` 切换
- [ ] FSM 状态变化 → 触发动画切换

---

## M4 — 完整版 ⏳

**目标：** 所有工具映射 + 窗口打磨 + 发布

**任务清单：**
- [ ] 补全所有工具状态动画资源
- [ ] always-on-top 窗口配置
- [ ] 透明背景（无边框）
- [ ] 断线自动重连
- [ ] 打包 Mac（`.dmg`）+ Windows（`.msi`）
- [ ] 安装说明文档

---

## 技术债务 & 后续优化（不在当前里程碑）

- 多角色/皮肤切换系统
- 历史事件回放
- 性能监控面板
- Lottie 作为 Spine 替代选项

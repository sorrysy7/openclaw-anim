/**
 * openclaw-anim plugin
 *
 * Captures before_tool_call / after_tool_call hooks and broadcasts
 * minimal, content-free events to connected SSE clients.
 *
 * Event shape: { ts, runId, phase, tool }
 * NO params, NO results, NO file paths, NO secrets — ever.
 */

import type { ServerResponse } from "node:http";
import { definePluginEntry } from "openclaw/plugin-sdk/plugin-entry";

// ─── SSE client registry ──────────────────────────────────────────────────────

const clients = new Set<ServerResponse>();

function broadcast(payload: object): void {
  if (clients.size === 0) return;
  const data = `data: ${JSON.stringify(payload)}\n\n`;
  for (const res of clients) {
    try {
      res.write(data);
    } catch {
      // Client already disconnected; will be removed on "close"
      clients.delete(res);
    }
  }
}

// ─── Plugin entry ──────────────────────────────────────────────────────────────

export default definePluginEntry({
  id: "openclaw-anim",
  name: "OpenClaw Anim",
  description:
    "Broadcasts tool call events via SSE for the animation desktop app.",

  register(api) {
    // ── 1. SSE endpoint ──────────────────────────────────────────────────────
    api.registerHttpRoute({
      path: "/api/anim/events",
      auth: "gateway", // requires Gateway Bearer Token
      match: "exact",
      handler(req, res) {
        res.setHeader("Content-Type", "text/event-stream");
        res.setHeader("Cache-Control", "no-cache");
        res.setHeader("Connection", "keep-alive");
        // Allow cross-origin from Tauri WebView (localhost)
        res.setHeader("Access-Control-Allow-Origin", "*");
        res.statusCode = 200;

        // Register client
        clients.add(res);
        api.logger.info(`[openclaw-anim] SSE client connected (total: ${clients.size})`);

        // Send an initial "connected" event so the client knows it's live
        res.write(`data: ${JSON.stringify({ type: "connected", ts: Date.now() })}\n\n`);

        // Heartbeat every 30s to prevent proxy/NAT timeout
        const heartbeat = setInterval(() => {
          try {
            res.write(": heartbeat\n\n");
          } catch {
            clearInterval(heartbeat);
          }
        }, 30_000);

        // Cleanup on disconnect
        req.on("close", () => {
          clients.delete(res);
          clearInterval(heartbeat);
          api.logger.info(`[openclaw-anim] SSE client disconnected (total: ${clients.size})`);
        });

        return true; // route handled
      },
    });

    // ── 2. Tool call hooks ───────────────────────────────────────────────────

    // before_tool_call → phase: "start"
    api.on("before_tool_call", (event, ctx) => {
      broadcast({
        ts: Date.now(),
        runId: ctx.runId ?? "unknown",
        phase: "start",
        tool: event.toolName,
        // Intentionally omitting event.params — no content, no secrets
      });
    });

    // after_tool_call → phase: "end" or "error"
    api.on("after_tool_call", (event, ctx) => {
      broadcast({
        ts: Date.now(),
        runId: ctx.runId ?? "unknown",
        phase: event.error ? "error" : "end",
        tool: event.toolName,
        // Intentionally omitting event.result / event.error message — no content
      });
    });

    api.logger.info("[openclaw-anim] Plugin registered — SSE endpoint: GET /api/anim/events");

    // ── 3. Outbound message hooks (drive "reply" animation) ───────────────
    // Note: we emit *no content*; only phases.
    api.on("message_sending", (_event, ctx) => {
      broadcast({
        ts: Date.now(),
        runId: ctx.runId ?? "unknown",
        phase: "reply_start",
        tool: "assistant_reply",
      });
    });

    api.on("message_sent", (_event, ctx) => {
      broadcast({
        ts: Date.now(),
        runId: ctx.runId ?? "unknown",
        phase: "reply_end",
        tool: "assistant_reply",
      });
    });
  },
});

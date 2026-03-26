import { listen } from "@tauri-apps/api/event";
import { SpineRenderer } from "./spine_renderer";

window.addEventListener("DOMContentLoaded", async () => {
  const root = document.querySelector<HTMLDivElement>("#app");
  const canvas = document.querySelector<HTMLCanvasElement>("#spine");
  if (!root || !canvas) return;

  root.innerHTML = `
    <div id="pet" style="font-family: system-ui; padding: 10px; user-select:none; -webkit-user-select:none;">
      <div id="status" style="opacity:0.7; font-size:12px; display:none;">loading spine...</div>
    </div>
  `;

  const statusEl = document.querySelector<HTMLDivElement>("#status")!;
  let showStatus = false;
  let initialSpine: string | undefined;

  const setStatus = (line: string) => {
    if (!showStatus) return;
    statusEl.textContent = line;
  };

  // Receive config from Rust
  await listen("anim://config", (evt) => {
    const cfg: any = evt.payload;
    showStatus = !!cfg?.showStatus;
    initialSpine = cfg?.initialSpine ?? undefined;
    statusEl.style.display = showStatus ? "block" : "none";
  });

  const spine = new SpineRenderer({
    canvas,
    atlasUrl: "/spine/chibi-stickers.atlas",
    jsonUrl: "/spine/chibi-stickers.json",
    // fallback if config not received yet
    initialAnimation: initialSpine ?? "emotes/just-right",
  });

  try {
    await spine.init();

    // Apply configured initial animation after init (most reliable)
    if (initialSpine) {
      spine.setAnimation(initialSpine, true, true);
    }

    setStatus("ready");
  } catch (e: any) {
    const msg = `spine init failed: ${String(e?.stack ?? e?.message ?? e)}`;
    setStatus(msg);

    // Send to Rust stdout so you can see it in the terminal running `tauri dev`.
    try {
      const { emit } = await import("@tauri-apps/api/event");
      await emit("anim://log", msg);
    } catch {
      // ignore
    }

    console.error("spine init failed", e);
  }

  // Native window dragging (recommended): delegate to Rust -> window.start_dragging().
  const startNativeDrag = async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("start_dragging");
  };

  // Drag lock toggle (double click)
  let dragLocked = false;
  window.addEventListener("dblclick", () => {
    dragLocked = !dragLocked;
    setStatus(dragLocked ? "drag: locked" : "drag: unlocked");
  });

  window.addEventListener("mousedown", async (e) => {
    // left button only
    if (e.button !== 0) return;
    if (dragLocked) return;

    // Avoid default browser selection/drag.
    e.preventDefault();
    e.stopPropagation();

    try {
      await startNativeDrag();
    } catch (err) {
      // If native drag fails, don't crash.
      console.warn("start_dragging failed", err);
    }
  });

  // No mousemove/mouseup needed when using native dragging.

  // Frontend receives ONLY sanitized events: { ts, action, spine, phase }
  await listen("anim://event", (evt) => {
    const payload: any = evt.payload;
    const ts = payload?.ts ?? Date.now();
    const spineKey = String(payload?.spine ?? payload?.action ?? "idle");
    spine.setAnimation(spineKey, true);
    setStatus(`[${new Date(ts).toLocaleTimeString()}] spine=${spineKey}`);
  });
});

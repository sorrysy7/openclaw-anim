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
  let spineAtlas = "/spine/chibi-stickers.atlas";  // default, overridden by config
  let spineJson  = "/spine/chibi-stickers.json";
  let spineSkin: string | undefined;
  let spineCamX: number | undefined;
  let spineCamY: number | undefined;
  let spineZoom: number | undefined;

  const setStatus = (line: string) => {
    if (!showStatus) return;
    statusEl.textContent = line;
  };

  // Wait for Rust config event (with 500ms timeout fallback)
  await new Promise<void>((resolve) => {
    const timer = setTimeout(resolve, 500);
    listen("anim://config", (evt) => {
      clearTimeout(timer);
      const cfg: any = evt.payload;
      showStatus = !!cfg?.showStatus;
      initialSpine = cfg?.initialSpine ?? undefined;
      if (cfg?.spineAtlas) spineAtlas = cfg.spineAtlas;
      if (cfg?.spineJson)  spineJson  = cfg.spineJson;
      spineSkin = cfg?.spineSkin ?? undefined;
      statusEl.style.display = showStatus ? "block" : "none";
      spineCamX = (cfg?.spineCamX !== undefined && cfg.spineCamX !== 0) ? cfg.spineCamX : spineCamX;
      spineCamY = (cfg?.spineCamY !== undefined && cfg.spineCamY !== 0) ? cfg.spineCamY : spineCamY;
      spineZoom = (cfg?.spineZoom !== undefined && cfg.spineZoom !== 0) ? cfg.spineZoom : spineZoom;
      console.log(`[config] skin=${spineSkin} camX=${spineCamX} camY=${spineCamY} zoom=${spineZoom}`);
      resolve();
    });
  });

  const spine = new SpineRenderer({
    canvas,
    atlasUrl: spineAtlas,
    jsonUrl:  spineJson,
    skin: spineSkin ?? "spineboy",
    // fallback if config not received yet
    initialAnimation: initialSpine ?? "emotes/idle",
    camX: spineCamX,
    camY: spineCamY,
    zoom: spineZoom,
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

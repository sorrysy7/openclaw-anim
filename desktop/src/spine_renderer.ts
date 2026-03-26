import {
  AssetManager,
  AtlasAttachmentLoader,
  SkeletonJson,
  Skeleton,
  AnimationStateData,
  AnimationState,
  Physics,
} from "@esotericsoftware/spine-webgl";
import { SceneRenderer } from "@esotericsoftware/spine-webgl";

export type SpineRendererOptions = {
  canvas: HTMLCanvasElement;
  atlasUrl: string;
  jsonUrl: string;
  initialAnimation?: string;
  /** Spine skin name to activate on init. Required for multi-skin assets like chibi-stickers. */
  skin?: string;
  /**
   * Camera config from config.json.
   * camX / camY: world-space center.
   * zoom: OrthoCamera zoom (>1 = zoom in, <1 = zoom out). 0 = auto-fit.
   */
  camX?: number;
  camY?: number;
  zoom?: number;
};

export class SpineRenderer {
  private opts: SpineRendererOptions;
  private canvas: HTMLCanvasElement;
  private gl: WebGLRenderingContext;
  private scene: SceneRenderer;
  private assets: AssetManager;

  private skeleton?: Skeleton;
  private state?: AnimationState;
  private currentAnim = "";

  // Skeleton world bounds — populated after skeleton data is loaded
  private skelBounds = { x: 0, y: 0, width: 360, height: 682.5 };

  constructor(opts: SpineRendererOptions) {
    this.opts = opts;
    this.canvas = opts.canvas;

    const gl = this.canvas.getContext("webgl", {
      alpha: true,
      premultipliedAlpha: true,
      antialias: true,
    }) as WebGLRenderingContext | null;
    if (!gl) throw new Error("WebGL not available");
    this.gl = gl;

    this.scene = new SceneRenderer(this.canvas, this.gl, true);
    this.assets = new AssetManager(this.gl);

    this.assets.loadTextureAtlas(opts.atlasUrl);
    this.assets.loadText(opts.jsonUrl);

    this.currentAnim = opts.initialAnimation ?? "idle";
  }

  async init(): Promise<void> {
    await this.waitForAssets();

    const log = async (msg: string) => {
      try {
        const { emit } = await import("@tauri-apps/api/event");
        await emit("anim://log", `[spine] ${msg}`);
      } catch { /* ignore */ }
    };

    await log("assets loaded");

    const atlas = this.assets.get(this.opts.atlasUrl);
    const jsonAny = this.assets.get(this.opts.jsonUrl) as any;

    if (!atlas) throw new Error("atlas not loaded");
    if (!jsonAny) throw new Error("json not loaded");

    let skeletonData;
    try {
      const atlasLoader = new AtlasAttachmentLoader(atlas);
      const json = new SkeletonJson(atlasLoader);
      skeletonData = json.readSkeletonData(jsonAny as string);
      await log("readSkeletonData ok");
    } catch (e: any) {
      await log(`readSkeletonData failed: ${String(e?.stack ?? e?.message ?? e)}`);
      throw e;
    }

    try {
      this.skeleton = new Skeleton(skeletonData);

      // Apply skin before setToSetupPose (multi-skin assets have no visible body in default skin)
      const skinName = this.opts.skin;
      if (skinName) {
        const skin = skeletonData.findSkin(skinName);
        if (skin) {
          this.skeleton.setSkin(skin);
          await log(`skin set: ${skinName}`);
        } else {
          const available = skeletonData.skins.map((s: any) => s.name).join(", ");
          await log(`skin "${skinName}" not found. Available: ${available}`);
        }
      } else {
        const available = skeletonData.skins.map((s: any) => s.name).join(", ");
        await log(`no skin specified. Available: ${available}`);
      }

      this.skeleton.setToSetupPose();
      this.skeleton.updateWorldTransform(Physics.update);

      // Read bounds exported by Spine (x/y = bottom-left, width/height = size)
      const sd = skeletonData as any;
      if (sd.width != null && sd.height != null) {
        this.skelBounds = {
          x: sd.x ?? 0,
          y: sd.y ?? 0,
          width: sd.width,
          height: sd.height,
        };
        await log(`skelBounds: x=${sd.x} y=${sd.y} w=${sd.width} h=${sd.height}`);
      }

      await log("skeleton init ok");
    } catch (e: any) {
      await log(`skeleton init failed: ${String(e?.stack ?? e?.message ?? e)}`);
      throw e;
    }

    const stateData = new AnimationStateData(skeletonData);
    stateData.defaultMix = 0.2;
    this.state = new AnimationState(stateData);

    // Dump animation list so user knows valid names
    const animNames = skeletonData.animations.map((a: any) => a.name).join(", ");
    await log(`animations: ${animNames}`);

    this.setAnimation(this.currentAnim, true, true);
    this.applyCamera();
    this.loop();
  }

  setAnimation(name: string, loop = true, force = false) {
    if (!this.skeleton || !this.state) return;
    if (!name) return;
    if (!force && name === this.currentAnim) return;
    this.currentAnim = name;

    const has = this.skeleton.data.findAnimation(name) != null;
    const fallback =
      this.skeleton.data.findAnimation("emotes/idle") != null
        ? "emotes/idle"
        : this.skeleton.data.animations[0]?.name ?? name;
    this.state.setAnimation(0, has ? name : fallback, loop);
  }

  /** Update camera params at runtime (called when config reloads or after init). */
  updateCamera(camX?: number, camY?: number, zoom?: number) {
    if (camX !== undefined) this.opts.camX = camX;
    if (camY !== undefined) this.opts.camY = camY;
    if (zoom !== undefined) this.opts.zoom = zoom;
  }

  /** Apply camera position/zoom to scene.camera. Called once after init and every frame. */
  private applyCamera() {
    const cam = this.scene.camera;
    const { x, y, width, height } = this.skelBounds;

    // Camera center: skeleton center by default
    cam.position.x = this.opts.camX !== undefined ? this.opts.camX : x + width / 2;
    cam.position.y = this.opts.camY !== undefined ? this.opts.camY : y + height / 2;

    if (this.opts.zoom && this.opts.zoom !== 0) {
      // Direct OrthoCamera zoom: larger value = more world shown = smaller character.
      // To fit skeleton height into canvas: zoom ≈ skelHeight / canvasHeight
      // e.g. skelHeight=682.5, canvas=320 → zoom≈2.13 fits the character
      cam.zoom = this.opts.zoom;
    } else {
      // Auto-fit: compute zoom so skeleton fills canvas with 10% margin
      // OrthoCamera shows (zoom * canvasW) x (zoom * canvasH) world units
      // → zoom = skelDimension / (canvasDimension * PADDING)
      const PADDING = 0.9;
      const canvasAspect = this.canvas.width / this.canvas.height;
      const skelAspect = width / height;
      cam.zoom =
        canvasAspect > skelAspect
          ? height / (this.canvas.height * PADDING)   // height-constrained
          : width  / (this.canvas.width  * PADDING);  // width-constrained
    }

    cam.update();
  }

  private resize() {
    const dpr = window.devicePixelRatio || 1;
    const w = Math.round(this.canvas.clientWidth * dpr);
    const h = Math.round(this.canvas.clientHeight * dpr);
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    this.gl.viewport(0, 0, w, h);
    this.scene.camera.setViewport(w, h);
  }

  private loop = () => {
    requestAnimationFrame(this.loop);
    if (!this.skeleton || !this.state) return;

    this.resize();

    const delta = 1 / 60;
    this.state.update(delta);
    this.state.apply(this.skeleton);
    this.skeleton.updateWorldTransform(Physics.update);

    this.gl.clearColor(0, 0, 0, 0);
    this.gl.clear(this.gl.COLOR_BUFFER_BIT);

    this.applyCamera();

    this.scene.begin();
    this.scene.drawSkeleton(this.skeleton);
    this.scene.end();
  };

  private async waitForAssets() {
    const log = async (msg: string) => {
      try {
        const { emit } = await import("@tauri-apps/api/event");
        await emit("anim://log", `[spine] ${msg}`);
      } catch { /* ignore */ }
    };

    await new Promise<void>((resolve, reject) => {
      const tick = async () => {
        if (this.assets.isLoadingComplete()) return resolve();
        if (this.assets.hasErrors()) {
          const errs: any = (this.assets as any).errors ?? this.assets.getErrors?.();
          const detail = (() => { try { return JSON.stringify(errs); } catch { return String(errs); } })();
          void log(`asset load errors: ${detail}`);
          return reject(new Error(detail));
        }
        requestAnimationFrame(() => { void tick(); });
      };
      void tick();
    });
  }
}

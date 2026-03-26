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

    this.scene = new SceneRenderer(this.canvas, this.gl);
    this.assets = new AssetManager(this.gl);

    this.assets.loadTextureAtlas(opts.atlasUrl);
    this.assets.loadText(opts.jsonUrl);

    this.currentAnim = opts.initialAnimation ?? "idle";

    window.addEventListener("resize", () => this.resize());
  }

  async init(): Promise<void> {
    await this.waitForAssets();

    // Emit debug logs to Rust stdout via `anim://log`
    const log = async (msg: string) => {
      try {
        const { emit } = await import("@tauri-apps/api/event");
        await emit("anim://log", `[spine] ${msg}`);
      } catch {
        // ignore
      }
    };

    await log("assets loaded");

    // AssetManager keys are the same strings you passed to loadTextureAtlas/loadText.
    const atlas = this.assets.get(this.opts.atlasUrl);
    await log(`atlas loaded=${!!atlas} key=${this.opts.atlasUrl}`);

    const jsonAny = this.assets.get(this.opts.jsonUrl) as any;
    await log(
      `json loaded=${!!jsonAny} key=${this.opts.jsonUrl} type=${typeof jsonAny} len=${
        typeof jsonAny === "string" ? jsonAny.length : -1
      }`,
    );

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
      this.skeleton.setToSetupPose();
      this.skeleton.updateWorldTransform(Physics.update);
      await log("skeleton init ok");
    } catch (e: any) {
      await log(`skeleton init failed: ${String(e?.stack ?? e?.message ?? e)}`);
      throw e;
    }

    const stateData = new AnimationStateData(skeletonData);
    stateData.defaultMix = 0.2;
    this.state = new AnimationState(stateData);

    // Start animation (if missing, fall back to idle)
    this.setAnimation(this.currentAnim, true, true);

    this.resize();
    this.loop();
  }

  setAnimation(name: string, loop = true, force = false) {
    if (!this.skeleton || !this.state) return;
    if (!name) return;

    if (!force && name === this.currentAnim) return;
    this.currentAnim = name;

    const has = this.skeleton.data.findAnimation(name) != null;
    this.state.setAnimation(0, has ? name : "idle", loop);
  }

  private async waitForAssets() {
    const log = async (msg: string) => {
      try {
        const { emit } = await import("@tauri-apps/api/event");
        await emit("anim://log", `[spine] ${msg}`);
      } catch {
        // ignore
      }
    };

    await new Promise<void>((resolve, reject) => {
      const tick = async () => {
        if (this.assets.isLoadingComplete()) return resolve();

        if (this.assets.hasErrors()) {
          // Try to dump as much as possible — often includes the failing URL.
          const errs: any = (this.assets as any).errors ?? this.assets.getErrors?.();
          const detail = (() => {
            try {
              return JSON.stringify(errs);
            } catch {
              return String(errs);
            }
          })();
          void log(`asset load errors: ${detail}`);
          return reject(new Error(detail));
        }

        requestAnimationFrame(() => {
          void tick();
        });
      };
      void tick();
    });
  }

  private resize() {
    // spine-webgl SceneRenderer handles DPR internally; choose a resize mode.
    this.scene.resize("fit" as any);
  }

  private loop = () => {
    requestAnimationFrame(this.loop);

    if (!this.skeleton || !this.state) return;

    const delta = 1 / 60;
    this.state.update(delta);
    this.state.apply(this.skeleton);
    this.skeleton.updateWorldTransform(Physics.update);

    this.scene.begin();
    this.gl.clearColor(0, 0, 0, 0);
    this.gl.clear(this.gl.COLOR_BUFFER_BIT);

    this.scene.drawSkeleton(this.skeleton);
    this.scene.end();
  };
}

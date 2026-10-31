import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import classicPlayerUrl from "../assets/models/classic-player.gltf?url";
import slimPlayerUrl from "../assets/models/slim-player.gltf?url";
import {
  applyCapeTexture,
  applySkinTexture,
  createTransparentTexture,
  clonePlayer,
  loadModel,
  loadSkinTexture,
} from "../lib/skin-rendering";
import { Icon } from "./Icon";
import { useStore } from "../store";

const BASE = "idle";
const RANDOM = ["idle_sub_1", "idle_sub_2", "idle_sub_3"] as const;
const FADE = 0.2;
const RANDOM_MS = 8000;
/** Frame interval with lighter graphics on. */
const LOW_FRAME_MS = 1000 / 30;

export function SkinPreview({
  skinUrl,
  capeUrl,
  model = "classic",
  name,
  bump = 0,
  reducedMotion = false,
  size = "md",
}: {
  skinUrl?: string | null;
  capeUrl?: string | null;
  model?: string;
  name?: string;
  bump?: number;
  reducedMotion?: boolean;
  /** md = modal / compact; lg = account hero */
  size?: "md" | "lg";
}) {
  const viewRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const pausedRef = useRef(false);
  const [ready, setReady] = useState(false);
  const [paused, setPaused] = useState(false);
  const low = !!useStore().settings?.reduceEffects;

  const api = useRef<{
    mixer: THREE.AnimationMixer | null;
    actions: Record<string, THREE.AnimationAction>;
    current: string;
    lastRandom: string;
    timer: number;
    root: THREE.Group | null;
    scene: THREE.Object3D | null;
    transparent: THREE.Texture;
    frameCamera: () => void;
    play: (name: string) => void;
    scheduleRandom: () => void;
    clearTimer: () => void;
    invalidate: () => void;
  } | null>(null);

  useEffect(() => {
    const view = viewRef.current;
    const canvas = canvasRef.current;
    if (!view || !canvas) return;

    const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: !low, powerPreference: low ? "low-power" : "default" });
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.NoToneMapping;
    renderer.setPixelRatio(low ? 1 : Math.min(window.devicePixelRatio, 2));

    const scene = new THREE.Scene();
    const large = size === "lg";
    const camera = new THREE.PerspectiveCamera(large ? 32 : 36, 1, 0.1, 100);
    camera.position.set(0, 1.1, -3.4);

    scene.add(new THREE.AmbientLight(0xffffff, 2));
    const dir = new THREE.DirectionalLight(0xffffff, 1.2);
    dir.position.set(-3, 4, -2);
    scene.add(dir);

    const root = new THREE.Group();
    root.rotation.y = Math.PI / 8 + Math.PI;
    scene.add(root);
    const lookAt = new THREE.Vector3(0, 1, 0);

    const transparent = createTransparentTexture();
    const clock = new THREE.Clock();
    let raf = 0;
    let disposed = false;
    let onScreen = true;
    let lastFrame = 0;
    const visible = () => onScreen && !document.hidden;
    // The idle animation needs every frame; otherwise draw only when something changed.
    const animating = () => !!state.mixer && !pausedRef.current && !reducedMotion;
    const invalidate = () => {
      if (!raf && !disposed && visible()) raf = requestAnimationFrame(tick);
    };

    const frameCamera = () => {
      if (!root.children.length) return;
      root.position.set(0, 0, 0);
      root.scale.setScalar(1);
      root.updateMatrixWorld(true);
      const box = new THREE.Box3().setFromObject(root);
      const size3 = new THREE.Vector3();
      const center = new THREE.Vector3();
      box.getSize(size3);
      box.getCenter(center);
      root.position.sub(center);
      root.position.y += size3.y * 0.04;
      root.updateMatrixWorld(true);
      box.setFromObject(root);
      box.getSize(size3);
      box.getCenter(center);
      const fov = (camera.fov * Math.PI) / 180;
      const fitH = size3.y / (2 * Math.tan(fov / 2));
      const fitW = size3.x / ((2 * Math.tan(fov / 2)) * Math.max(camera.aspect, 0.01));
      const dist = Math.max(fitH, fitW) * (large ? 1.45 : 1.55);
      // Keep a level look — slight bias up so feet aren't crushed by perspective.
      camera.position.set(center.x, center.y + size3.y * 0.02, center.z - dist);
      lookAt.set(center.x, center.y + size3.y * 0.02, center.z);
    };

    const state = {
      mixer: null as THREE.AnimationMixer | null,
      actions: {} as Record<string, THREE.AnimationAction>,
      current: "",
      lastRandom: "",
      timer: 0,
      root,
      scene: null as THREE.Object3D | null,
      transparent,
      frameCamera,
      invalidate: () => invalidate(),
      play(name: string) {
        const action = state.actions[name];
        if (!state.mixer || !action) return;
        Object.entries(state.actions).forEach(([n, a]) => {
          if (n !== name && a.isRunning()) a.fadeOut(FADE);
        });
        action.reset();
        if (name === BASE) {
          action.setLoop(THREE.LoopRepeat, Infinity);
        } else {
          action.setLoop(THREE.LoopOnce, 1);
          action.clampWhenFinished = true;
          const onFinished = (event: { action: THREE.AnimationAction }) => {
            if (event.action !== action) return;
            state.mixer?.removeEventListener("finished", onFinished as never);
            if (state.current !== name || !state.actions[BASE]) return;
            action.fadeOut(FADE);
            const base = state.actions[BASE];
            base.reset().fadeIn(FADE).play();
            state.current = BASE;
            if (!pausedRef.current) state.scheduleRandom();
          };
          state.mixer.addEventListener("finished", onFinished as never);
        }
        action.fadeIn(FADE).play();
        state.current = name;
      },
      clearTimer() {
        if (state.timer) {
          window.clearTimeout(state.timer);
          state.timer = 0;
        }
      },
      scheduleRandom() {
        state.clearTimer();
        if (reducedMotion || pausedRef.current) return;
        state.timer = window.setTimeout(() => {
          if (pausedRef.current || state.current !== BASE) {
            state.scheduleRandom();
            return;
          }
          const pool = RANDOM.filter((n) => n !== state.lastRandom);
          const pick = (pool.length ? pool : RANDOM)[Math.floor(Math.random() * (pool.length || RANDOM.length))];
          if (state.actions[pick]) {
            state.lastRandom = pick;
            state.play(pick);
          } else {
            state.scheduleRandom();
          }
        }, RANDOM_MS);
      },
    };
    api.current = state;

    const resize = () => {
      const w = Math.max(1, view.clientWidth || (large ? 360 : 280));
      const h = Math.max(1, view.clientHeight || (large ? 400 : 360));
      // updateStyle false + CSS 100% — aspect must match the stage or the model looks stretched.
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      frameCamera();
      invalidate();
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(view);

    let dragging = false;
    let prevX = 0;
    let dragged = false;

    const onDown = (e: PointerEvent) => {
      canvas.setPointerCapture(e.pointerId);
      dragging = true;
      prevX = e.clientX;
      dragged = false;
    };
    const onMove = (e: PointerEvent) => {
      if (!dragging) return;
      root.rotation.y += (e.clientX - prevX) * 0.01;
      prevX = e.clientX;
      dragged = true;
      invalidate();
    };
    const onUp = (e: PointerEvent) => {
      dragging = false;
      try { canvas.releasePointerCapture(e.pointerId); } catch { /* */ }
    };
    const onClick = () => {
      if (dragged || pausedRef.current || reducedMotion) {
        dragged = false;
        return;
      }
      if (state.actions.interact) state.play("interact");
      dragged = false;
      invalidate();
    };

    canvas.addEventListener("pointerdown", onDown);
    canvas.addEventListener("pointermove", onMove);
    canvas.addEventListener("pointerup", onUp);
    canvas.addEventListener("pointerleave", onUp);
    canvas.addEventListener("click", onClick);

    function tick(now: number) {
      raf = 0;
      if (disposed || !visible()) return;
      if (low && animating() && now - lastFrame < LOW_FRAME_MS) {
        raf = requestAnimationFrame(tick);
        return;
      }
      lastFrame = now;
      // Clamp so the animation does not jump after the view was hidden.
      const dt = Math.min(clock.getDelta(), 0.1);
      if (state.mixer && !pausedRef.current) state.mixer.update(dt);
      camera.lookAt(lookAt);
      renderer.render(scene, camera);
      if (animating()) raf = requestAnimationFrame(tick);
    }
    const io = new IntersectionObserver(([entry]) => {
      onScreen = entry.isIntersecting;
      clock.getDelta();
      invalidate();
    });
    io.observe(view);
    const onVisibility = () => { clock.getDelta(); invalidate(); };
    document.addEventListener("visibilitychange", onVisibility);
    invalidate();

    return () => {
      disposed = true;
      cancelAnimationFrame(raf);
      state.clearTimer();
      ro.disconnect();
      io.disconnect();
      document.removeEventListener("visibilitychange", onVisibility);
      canvas.removeEventListener("pointerdown", onDown);
      canvas.removeEventListener("pointermove", onMove);
      canvas.removeEventListener("pointerup", onUp);
      canvas.removeEventListener("pointerleave", onUp);
      canvas.removeEventListener("click", onClick);
      state.mixer?.stopAllAction();
      renderer.dispose();
      transparent.dispose();
      api.current = null;
    };
  }, [reducedMotion, size, low]);

  useEffect(() => {
    const state = api.current;
    if (!state) return;
    let cancelled = false;
    setReady(false);

    (async () => {
      try {
        const modelUrl = model === "slim" ? slimPlayerUrl : classicPlayerUrl;
        const gltf = await loadModel(modelUrl);
        if (cancelled || !api.current) return;

        while (state.root!.children.length) {
          state.root!.remove(state.root!.children[0]);
        }

        const player = clonePlayer(gltf.scene);
        state.root!.add(player);
        state.scene = player;

        state.mixer?.stopAllAction();
        state.mixer = new THREE.AnimationMixer(player);
        state.actions = {};
        for (const clip of gltf.animations) {
          const action = state.mixer.clipAction(clip);
          action.setLoop(THREE.LoopOnce, 1);
          action.clampWhenFinished = true;
          state.actions[clip.name] = action;
        }

        if (skinUrl) {
          const tex = await loadSkinTexture(skinUrl);
          if (cancelled || !api.current) return;
          applySkinTexture(player, tex);
        }
        applyCapeTexture(player, null, state.transparent);
        state.frameCamera();

        if (!reducedMotion && state.actions[BASE]) {
          state.play(BASE);
          if (!pausedRef.current) state.scheduleRandom();
        }

        state.invalidate();
        if (!cancelled) setReady(true);
      } catch {
        if (!cancelled) setReady(true);
      }
    })();

    return () => { cancelled = true; };
  }, [skinUrl, model, reducedMotion, low]);

  useEffect(() => {
    const state = api.current;
    if (!state?.scene) return;
    let cancelled = false;
    (async () => {
      try {
        if (capeUrl) {
          const cape = await loadSkinTexture(capeUrl);
          if (cancelled || !api.current?.scene) return;
          applyCapeTexture(api.current.scene, cape, api.current.transparent);
        } else {
          applyCapeTexture(state.scene!, null, state.transparent);
        }
        api.current?.frameCamera();
        api.current?.invalidate();
      } catch { /* */ }
    })();
    return () => { cancelled = true; };
  }, [capeUrl, ready]);

  useEffect(() => {
    pausedRef.current = paused;
    const state = api.current;
    if (!state) return;
    state.invalidate();
    if (paused) {
      state.clearTimer();
      return;
    }
    if (state.current === BASE || !state.current) {
      if (state.actions[BASE] && state.current !== BASE) state.play(BASE);
      state.scheduleRandom();
    }
  }, [paused]);

  useEffect(() => {
    const state = api.current;
    if (!state?.root || reducedMotion || bump === 0) return;
    const start = performance.now();
    const from = state.root.rotation.y;
    const extra = Math.PI * 1.15;
    let raf = 0;
    const tick = (t: number) => {
      const p = Math.min(1, (t - start) / 520);
      state.root!.rotation.y = from + extra * (1 - Math.pow(1 - p, 3));
      state.invalidate();
      if (p < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    if (!pausedRef.current && state.actions.interact) state.play("interact");
    return () => cancelAnimationFrame(raf);
  }, [bump, reducedMotion]);

  return (
    <div className={`skin-shell skin-shell-${size}${ready ? " is-ready" : ""}`}>
      {/* 3D view only — dock stays outside this box */}
      <div className="skin-stage" ref={viewRef}>
        <canvas ref={canvasRef} className="skin-canvas" aria-label={name ? `${name} skin preview` : "Skin preview"} />
      </div>
      <div className="skin-dock">
        <p className="skin-hint">Drag to rotate</p>
        {!reducedMotion ? (
          <button
            type="button"
            className="skin-pause"
            aria-pressed={paused}
            aria-label={paused ? "Resume animation" : "Pause animation"}
            title={paused ? "Resume" : "Pause"}
            onClick={() => setPaused((p) => !p)}
          >
            <Icon name={paused ? "play" : "pause"} size={16} />
          </button>
        ) : (
          <span aria-hidden="true" />
        )}
      </div>
    </div>
  );
}

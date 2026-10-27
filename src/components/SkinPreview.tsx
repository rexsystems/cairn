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

const BASE = "idle";
const RANDOM = ["idle_sub_1", "idle_sub_2", "idle_sub_3"] as const;
const FADE = 0.2;
const RANDOM_MS = 8000;

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

  const api = useRef<{
    mixer: THREE.AnimationMixer | null;
    actions: Record<string, THREE.AnimationAction>;
    current: string;
    lastRandom: string;
    timer: number;
    root: THREE.Group | null;
    scene: THREE.Object3D | null;
    transparent: THREE.Texture;
    play: (name: string) => void;
    scheduleRandom: () => void;
    clearTimer: () => void;
  } | null>(null);

  useEffect(() => {
    const view = viewRef.current;
    const canvas = canvasRef.current;
    if (!view || !canvas) return;

    const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.NoToneMapping;
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

    const scene = new THREE.Scene();
    const large = size === "lg";
    // Keep camera at chest height relative to the model so lowering doesn't tilt into a top-down shot.
    const modelY = large ? -0.55 : -0.42;
    const camera = new THREE.PerspectiveCamera(large ? 34 : 38, 1, 0.1, 100);
    camera.position.set(0, modelY + 1.32, large ? -2.75 : -3.05);

    scene.add(new THREE.AmbientLight(0xffffff, 2));
    const dir = new THREE.DirectionalLight(0xffffff, 1.2);
    dir.position.set(-3, 4, -2);
    scene.add(dir);

    const root = new THREE.Group();
    root.position.set(0, modelY, large ? 1.7 : 1.9);
    root.scale.setScalar(large ? 1.16 : 1.06);
    root.rotation.y = Math.PI / 8 + Math.PI;
    scene.add(root);

    const transparent = createTransparentTexture();
    const clock = new THREE.Clock();
    let raf = 0;
    let disposed = false;

    const state = {
      mixer: null as THREE.AnimationMixer | null,
      actions: {} as Record<string, THREE.AnimationAction>,
      current: "",
      lastRandom: "",
      timer: 0,
      root,
      scene: null as THREE.Object3D | null,
      transparent,
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
      const w = view.clientWidth || (large ? 360 : 280);
      const h = view.clientHeight || (large ? 480 : 360);
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
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
    };

    canvas.addEventListener("pointerdown", onDown);
    canvas.addEventListener("pointermove", onMove);
    canvas.addEventListener("pointerup", onUp);
    canvas.addEventListener("pointerleave", onUp);
    canvas.addEventListener("click", onClick);

    const tick = () => {
      if (disposed) return;
      const dt = clock.getDelta();
      if (state.mixer && !pausedRef.current) state.mixer.update(dt);
      const target = new THREE.Vector3(0, large ? 0.85 : 0.9, 0);
      if (state.scene) {
        const box = new THREE.Box3().setFromObject(state.root!);
        const c = new THREE.Vector3();
        box.getCenter(c);
        target.set(c.x, c.y - box.getSize(new THREE.Vector3()).y * 0.06, c.z);
      }
      camera.lookAt(target);
      renderer.render(scene, camera);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);

    return () => {
      disposed = true;
      cancelAnimationFrame(raf);
      state.clearTimer();
      ro.disconnect();
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
  }, [reducedMotion, size]);

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

        if (!reducedMotion && state.actions[BASE]) {
          state.play(BASE);
          if (!pausedRef.current) state.scheduleRandom();
        }

        if (!cancelled) setReady(true);
      } catch {
        if (!cancelled) setReady(true);
      }
    })();

    return () => { cancelled = true; };
  }, [skinUrl, model, reducedMotion]);

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
      } catch { /* */ }
    })();
    return () => { cancelled = true; };
  }, [capeUrl, ready]);

  useEffect(() => {
    pausedRef.current = paused;
    const state = api.current;
    if (!state) return;
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

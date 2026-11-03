import { useEffect, useRef } from "react";
import { frameAt } from "../lib/cosmetics";

/** Front face of a cape (10×16 at 1,1 per 64×32 frame), animated the same way the mod plays strips. */
export function CapePreview({ src, frames, frameMs, size = 80, label }: { src: string; frames: number; frameMs: number; size?: number; label?: string }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const canvas = ref.current;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx) return;
    const img = new Image();
    let raf = 0;
    let last = -1;
    const start = performance.now();
    const draw = (now: number) => {
      const frame = frameAt(now - start, frames, frameMs);
      if (frame !== last && img.naturalWidth) {
        last = frame;
        const unit = img.naturalWidth / 64;
        const frameH = img.naturalWidth / 2;
        ctx.imageSmoothingEnabled = false;
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.drawImage(img, unit, frame * frameH + unit, 10 * unit, 16 * unit, 0, 0, canvas.width, canvas.height);
      }
      if (frames > 1) raf = requestAnimationFrame(draw);
    };
    img.onload = () => { raf = requestAnimationFrame(draw); };
    img.src = src;
    return () => { cancelAnimationFrame(raf); img.onload = null; };
  }, [src, frames, frameMs]);
  return <canvas ref={ref} className="cape-preview" width={size} height={Math.round(size * 1.6)} role={label ? "img" : undefined} aria-label={label} />;
}

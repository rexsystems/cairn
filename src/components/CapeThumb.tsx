import { useEffect, useState } from "react";

/** Mojang cape atlas is 64×32; front face is 10×16 at (1,1). */
const ATLAS_W = 64;
const ATLAS_H = 32;
const FACE_X = 1;
const FACE_Y = 1;
const FACE_W = 10;
const FACE_H = 16;
const VIEW_W = 64;
const VIEW_H = Math.round((VIEW_W * FACE_H) / FACE_W);

/**
 * Shows the cape front, not the raw atlas. Uses pixel offsets (CSS % background-position
 * does not act as a simple crop when the image is larger than the box).
 */
export function CapeThumb({ url, label }: { url: string; label?: string }) {
  const [failed, setFailed] = useState(false);
  const scale = VIEW_W / FACE_W;

  useEffect(() => { setFailed(false); }, [url]);

  return (
    <span
      className="cape-face"
      role={label ? "img" : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
    >
      {!failed && (
        <img
          src={url}
          alt=""
          draggable={false}
          width={ATLAS_W * scale}
          height={ATLAS_H * scale}
          onError={() => setFailed(true)}
          style={{
            position: "absolute",
            left: -FACE_X * scale,
            top: -FACE_Y * scale,
            width: ATLAS_W * scale,
            height: ATLAS_H * scale,
            maxWidth: "none",
            imageRendering: "pixelated",
          }}
        />
      )}
    </span>
  );
}

export const CAPE_THUMB_SIZE = { w: VIEW_W, h: VIEW_H };

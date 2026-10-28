import { useEffect, useState } from "react";

/** Minecraft skin atlas face is 8×8 at (8, 8) on a 64×64 sheet. */
const ATLAS = 64;
const FACE = 8;
const FACE_X = 8;
const FACE_Y = 8;
const VIEW = 48;

/** Face crop from a full skin PNG — same idea as CapeThumb. */
export function SkinThumb({ url, label }: { url: string; label?: string }) {
  const [failed, setFailed] = useState(false);
  const scale = VIEW / FACE;

  useEffect(() => {
    setFailed(false);
  }, [url]);

  return (
    <span
      className="skin-face"
      role={label ? "img" : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
    >
      {!failed && (
        <img
          src={url}
          alt=""
          draggable={false}
          width={ATLAS * scale}
          height={ATLAS * scale}
          onError={() => setFailed(true)}
          style={{
            position: "absolute",
            left: -FACE_X * scale,
            top: -FACE_Y * scale,
            width: ATLAS * scale,
            height: ATLAS * scale,
            maxWidth: "none",
            imageRendering: "pixelated",
          }}
        />
      )}
    </span>
  );
}

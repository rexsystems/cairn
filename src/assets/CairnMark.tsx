import markUrl from "./cairn-mark.svg?url";

/** Pixel cairn mark — stacked stones + play, matches the name. */
export function CairnMark({ className, size = 32 }: { className?: string; size?: number }) {
  return (
    <img
      className={className}
      src={markUrl}
      width={size}
      height={size}
      alt=""
      draggable={false}
      style={{ imageRendering: "pixelated" }}
    />
  );
}

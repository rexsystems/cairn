import { useEffect, useLayoutEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import { createPortal } from "react-dom";

type Pos = { left: number; top?: number; bottom?: number; placement: "up" | "down" };

function place(anchor: DOMRect, menuH: number): Pos {
  const gap = 6;
  const width = 260;
  const left = Math.min(Math.max(8, anchor.right - width), window.innerWidth - width - 8);
  const below = window.innerHeight - anchor.bottom - gap - 8;
  const above = anchor.top - gap - 8;
  const openUp = below < Math.min(menuH, 160) && above > below;
  if (openUp) {
    return { left, bottom: window.innerHeight - anchor.top + gap, placement: "up" };
  }
  return { left, top: anchor.bottom + gap, placement: "down" };
}

export function PopoverMenu({
  open,
  onClose,
  anchorRef,
  children,
}: {
  open: boolean;
  onClose: () => void;
  anchorRef: RefObject<HTMLElement | null>;
  children: ReactNode;
}) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<Pos | null>(null);

  useLayoutEffect(() => {
    if (!open || !anchorRef.current) return;
    const sync = () => {
      const a = anchorRef.current!.getBoundingClientRect();
      const h = menuRef.current?.offsetHeight ?? 220;
      setPos(place(a, h));
    };
    sync();
    // Re-measure after paint so real menu height drives flip.
    requestAnimationFrame(sync);
    window.addEventListener("resize", sync);
    window.addEventListener("scroll", sync, true);
    return () => {
      window.removeEventListener("resize", sync);
      window.removeEventListener("scroll", sync, true);
    };
  }, [open, anchorRef]);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      const t = e.target as Node;
      if (menuRef.current?.contains(t) || anchorRef.current?.contains(t)) return;
      onClose();
    };
    const onEsc = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      onClose();
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onEsc, true);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onEsc, true);
    };
  }, [open, onClose, anchorRef]);

  if (!open || !pos) return null;

  return createPortal(
    <div
      ref={menuRef}
      className={`popover popover-fixed popover-${pos.placement}`}
      role="menu"
      style={{ left: pos.left, top: pos.top, bottom: pos.bottom, minWidth: 240 }}
    >
      {children}
    </div>,
    document.body,
  );
}

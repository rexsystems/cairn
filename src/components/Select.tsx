import { useEffect, useId, useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { createPortal } from "react-dom";
import { Icon } from "./Icon";

export type SelectOption = { value: string; label: string; disabled?: boolean };

const MENU_MAX = 280;
const GAP = 6;

type MenuPos = { left: number; width: number; top?: number; bottom?: number; maxHeight: number; placement: "up" | "down" };

function placeMenu(trigger: DOMRect, optionCount: number): MenuPos {
  const width = Math.max(trigger.width, 140);
  const left = Math.min(Math.max(8, trigger.left), window.innerWidth - width - 8);
  const est = Math.min(MENU_MAX, Math.max(44, optionCount * 40 + 12));
  const below = window.innerHeight - trigger.bottom - GAP - 8;
  const above = trigger.top - GAP - 8;
  const openUp = below < Math.min(est, 160) && above > below;
  if (openUp) {
    return { left, width, bottom: window.innerHeight - trigger.top + GAP, maxHeight: Math.min(MENU_MAX, Math.max(120, above)), placement: "up" };
  }
  return { left, width, top: trigger.bottom + GAP, maxHeight: Math.min(MENU_MAX, Math.max(120, below)), placement: "down" };
}

export function Select({
  value,
  onChange,
  options,
  disabled,
  "aria-label": ariaLabel,
  className,
  style,
  placeholder = "Choose…",
}: {
  value: string;
  onChange: (value: string) => void;
  options: SelectOption[];
  disabled?: boolean;
  "aria-label"?: string;
  className?: string;
  style?: CSSProperties;
  placeholder?: string;
}) {
  const listId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const btnRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState<MenuPos | null>(null);
  const [active, setActive] = useState(-1);
  const typeBuf = useRef({ s: "", t: 0 });

  const selected = options.find((o) => o.value === value);
  const label = selected?.label ?? (value ? value : placeholder);

  const syncPos = () => {
    const el = btnRef.current;
    if (!el) return;
    setPos(placeMenu(el.getBoundingClientRect(), options.length));
  };

  useLayoutEffect(() => {
    if (!open) return;
    syncPos();
    const idx = Math.max(0, options.findIndex((o) => o.value === value && !o.disabled));
    setActive(idx >= 0 ? idx : options.findIndex((o) => !o.disabled));
    const onWin = () => syncPos();
    window.addEventListener("resize", onWin);
    window.addEventListener("scroll", onWin, true);
    return () => {
      window.removeEventListener("resize", onWin);
      window.removeEventListener("scroll", onWin, true);
    };
  }, [open, options, value]);

  useEffect(() => {
    if (open && pos) listRef.current?.focus({ preventScroll: true });
  }, [open, pos]);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      const t = e.target as Node;
      if (rootRef.current?.contains(t) || listRef.current?.contains(t)) return;
      setOpen(false);
    };
    // Capture on window so Escape closes the menu before a parent Modal handles it.
    const onEsc = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      setOpen(false);
      btnRef.current?.focus();
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onEsc, true);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onEsc, true);
    };
  }, [open]);

  useEffect(() => {
    if (!open || active < 0) return;
    const el = listRef.current?.querySelector<HTMLElement>(`[data-idx="${active}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [open, active]);

  const pick = (v: string) => {
    onChange(v);
    setOpen(false);
    btnRef.current?.focus();
  };

  const move = (dir: 1 | -1) => {
    if (!options.length) return;
    let i = active;
    for (let n = 0; n < options.length; n++) {
      i = (i + dir + options.length) % options.length;
      if (!options[i].disabled) { setActive(i); return; }
    }
  };

  const typeJump = (ch: string) => {
    const now = Date.now();
    if (now - typeBuf.current.t > 700) typeBuf.current.s = "";
    typeBuf.current.t = now;
    typeBuf.current.s += ch.toLowerCase();
    const q = typeBuf.current.s;
    const i = options.findIndex((o) => !o.disabled && o.label.toLowerCase().startsWith(q));
    if (i >= 0) setActive(i);
  };

  const onBtnKey = (e: ReactKeyboardEvent) => {
    if (disabled) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (!open) { setOpen(true); return; }
      if (e.key === "ArrowDown") move(1);
      else if (e.key === "ArrowUp") move(-1);
      else if ((e.key === "Enter" || e.key === " ") && active >= 0 && !options[active]?.disabled) pick(options[active].value);
    } else if (e.key === "Escape" && open) {
      e.preventDefault();
      setOpen(false);
    } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      if (!open) setOpen(true);
      typeJump(e.key);
    }
  };

  const onListKey = (e: ReactKeyboardEvent) => {
    if (e.key === "ArrowDown") { e.preventDefault(); move(1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); move(-1); }
    else if (e.key === "Home") { e.preventDefault(); setActive(options.findIndex((o) => !o.disabled)); }
    else if (e.key === "End") {
      e.preventDefault();
      for (let i = options.length - 1; i >= 0; i--) if (!options[i].disabled) { setActive(i); break; }
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (active >= 0 && !options[active]?.disabled) pick(options[active].value);
    } else if (e.key === "Escape" || e.key === "Tab") {
      setOpen(false);
      if (e.key === "Escape") { e.preventDefault(); btnRef.current?.focus(); }
    } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      typeJump(e.key);
    }
  };

  const menu = open && pos && createPortal(
    <div
      ref={listRef}
      id={listId}
      className={`select-menu select-menu-${pos.placement}`}
      role="listbox"
      tabIndex={-1}
      style={{
        left: pos.left,
        width: pos.width,
        maxHeight: pos.maxHeight,
        top: pos.top,
        bottom: pos.bottom,
      }}
      onKeyDown={onListKey}
    >
      {options.length === 0 ? (
        <div className="select-empty muted">No options</div>
      ) : options.map((o, i) => (
        <button
          key={`${o.value}-${i}`}
          type="button"
          role="option"
          data-idx={i}
          aria-selected={o.value === value}
          aria-disabled={o.disabled || undefined}
          disabled={o.disabled}
          className={`select-option${o.value === value ? " is-selected" : ""}${i === active ? " is-active" : ""}`}
          onMouseEnter={() => !o.disabled && setActive(i)}
          onClick={() => !o.disabled && pick(o.value)}
        >
          <span className="select-option-label">{o.label}</span>
          {o.value === value && <Icon name="check" size={14} />}
        </button>
      ))}
    </div>,
    document.body,
  );

  return (
    <div ref={rootRef} className={`select-root${className ? ` ${className}` : ""}`} style={style}>
      <button
        ref={btnRef}
        type="button"
        className="select-trigger"
        role="combobox"
        aria-expanded={open}
        aria-controls={listId}
        aria-haspopup="listbox"
        aria-label={ariaLabel}
        aria-disabled={disabled || undefined}
        disabled={disabled}
        onClick={() => !disabled && setOpen((v) => !v)}
        onKeyDown={onBtnKey}
      >
        <span className={`select-value${!selected ? " is-placeholder" : ""}`}>{label}</span>
        <span className={`select-chevron${open ? " is-open" : ""}`} aria-hidden="true"><Icon name="chevron" size={14} /></span>
      </button>
      {menu}
    </div>
  );
}

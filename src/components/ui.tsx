import { useEffect, useRef, type ReactNode } from "react";
import { Icon, type IconName } from "./Icon";
import { fileSrc, type Account } from "../api";

export function Modal({ title, onClose, children, footer, wide, labelledBy }: {
  title: ReactNode; onClose: () => void; children: ReactNode; footer?: ReactNode; wide?: boolean; labelledBy?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const titleId = labelledBy ?? "modal-title";
  useEffect(() => {
    const prev = document.activeElement as HTMLElement | null;
    const el = ref.current;
    const focusables = () => Array.from(el?.querySelectorAll<HTMLElement>('button, [href], input, select, textarea, [role="combobox"], [tabindex]:not([tabindex="-1"])') ?? []).filter((x) => !x.hasAttribute("disabled"));
    (focusables().find((x) => x.dataset.autofocus !== undefined) ?? focusables()[1] ?? focusables()[0])?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { e.stopPropagation(); onClose(); }
      if (e.key === "Tab") {
        const f = focusables();
        if (!f.length) return;
        const first = f[0], last = f[f.length - 1];
        if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
        else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
      }
    };
    document.addEventListener("keydown", onKey, true);
    return () => { document.removeEventListener("keydown", onKey, true); prev?.focus?.(); };
  }, [onClose]);
  return (
    <div className="modal-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className={`modal ${wide ? "modal-wide" : ""}`} role="dialog" aria-modal="true" aria-labelledby={titleId} ref={ref}>
        <div className="modal-head">
          <h2 id={titleId} style={{ flex: 1, minWidth: 0 }}>{title}</h2>
          <button className="btn btn-quiet btn-icon" onClick={onClose} aria-label="Close dialog"><Icon name="close" /></button>
        </div>
        <div className="modal-body">{children}</div>
        {footer && <div className="modal-foot">{footer}</div>}
      </div>
    </div>
  );
}

export function Loading({ label }: { label: string }) {
  return (
    <div className="state" role="status" aria-live="polite">
      <span className="loading"><Slabs />{label}</span>
    </div>
  );
}

export function Slabs() {
  return <span className="slabs" aria-hidden="true"><i /><i /><i /></span>;
}

export function EmptyState({ title, children, action, icon = "box" }: { title: string; children?: ReactNode; action?: ReactNode; icon?: IconName }) {
  return (
    <div className="state">
      <Icon name={icon} size={36} />
      <h3>{title}</h3>
      {children && <p>{children}</p>}
      {action}
    </div>
  );
}

export function ErrorState({ title, error, onRetry }: { title: string; error: string; onRetry?: () => void }) {
  return (
    <div className="state error" role="alert">
      <Icon name="warn" size={36} />
      <h3>{title}</h3>
      <p>{error}</p>
      {onRetry && <button className="btn" onClick={onRetry}><Icon name="update" />Try again</button>}
    </div>
  );
}

export function InstanceIcon({ name, src, large, small }: { name: string; src?: string | null; large?: boolean; small?: boolean }) {
  return (
    <div className={`inst-icon ${large ? "inst-icon-lg" : ""} ${small ? "inst-icon-sm" : ""}`} aria-hidden="true">
      {src ? <img src={fileSrc(src)} alt="" /> : <span className="inst-initial">{name.trim().charAt(0).toUpperCase() || "?"}</span>}
    </div>
  );
}

/** Face from the player's real skin texture (8x8 face + hat layer); initials for offline accounts. */
export function Avatar({ account, small }: { account?: Account | null; small?: boolean }) {
  const size = small ? 32 : 48;
  if (account?.skinUrl) {
    const scale = size / 8;
    const style = (x: number) => ({
      backgroundImage: `url(${account.skinUrl})`,
      backgroundSize: `${64 * scale}px auto`,
      backgroundPosition: `-${x * scale}px -${8 * scale}px`,
    });
    return (
      <div className={`avatar ${small ? "avatar-sm" : ""}`} aria-hidden="true">
        <span className="face" style={style(8)} />
        <span className="hat" style={style(40)} />
      </div>
    );
  }
  return <div className={`avatar ${small ? "avatar-sm" : ""}`} aria-hidden="true">{account?.username?.charAt(0).toUpperCase() ?? "?"}</div>;
}

export function Switch({ checked, onChange, label, disabled }: { checked: boolean; onChange: (v: boolean) => void; label: ReactNode; disabled?: boolean }) {
  return (
    <label className="switch">
      <input type="checkbox" role="switch" checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} />
      <span className="track" aria-hidden="true" />
      <span>{label}</span>
    </label>
  );
}

export { Select, type SelectOption } from "./Select";
export { PopoverMenu } from "./PopoverMenu";

export function Progress({ done, total }: { done: number; total: number }) {
  // Three slabs fill in order; the motif from DESIGN.md.
  const indeterminate = total <= 0;
  const frac = indeterminate ? 0 : Math.min(1, done / total);
  return (
    <div className={`progress ${indeterminate ? "indeterminate" : ""}`} role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={indeterminate ? undefined : Math.round(frac * 100)}>
      {[0, 1, 2].map((i) => (
        <span key={i}><i style={indeterminate ? undefined : { width: `${Math.max(0, Math.min(1, frac * 3 - i)) * 100}%` }} /></span>
      ))}
    </div>
  );
}

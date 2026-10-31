/** Basic gamepad focus navigation for the launcher UI. */
export function startGamepadNav(enabled: () => boolean): () => void {
  let raf = 0;
  let last = 0;
  const dead = 0.45;
  const cooldown = 220;

  const focusables = () =>
    Array.from(document.querySelectorAll<HTMLElement>('button:not([disabled]), a[href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'))
      .filter((el) => el.offsetParent !== null && !el.closest("[aria-hidden=true]"));

  const move = (dx: number, dy: number) => {
    const list = focusables();
    const cur = document.activeElement as HTMLElement | null;
    const i = cur ? list.indexOf(cur) : -1;
    if (!list.length) return;
    if (i < 0) { list[0].focus(); return; }
    const from = list[i].getBoundingClientRect();
    const cx = from.left + from.width / 2;
    const cy = from.top + from.height / 2;
    let best: HTMLElement | null = null;
    let bestScore = Infinity;
    for (const el of list) {
      if (el === list[i]) continue;
      const r = el.getBoundingClientRect();
      const ex = r.left + r.width / 2 - cx;
      const ey = r.top + r.height / 2 - cy;
      if (dx && Math.sign(ex) !== dx) continue;
      if (dy && Math.sign(ey) !== dy) continue;
      const along = Math.abs(dx ? ex : ey);
      const across = Math.abs(dx ? ey : ex);
      const score = along + across * 2;
      if (score < bestScore) { bestScore = score; best = el; }
    }
    (best ?? list[(i + (dx || dy) + list.length) % list.length])?.focus();
  };

  const connected = () => (navigator.getGamepads?.() ?? []).some((p) => p);

  // Polls only while a pad is connected and the window is visible; an idle frame loop
  // keeps WebView2 busy for nothing.
  const tick = () => {
    raf = 0;
    if (!enabled() || document.hidden) return;
    const g = (navigator.getGamepads?.() ?? []).find((p) => p);
    if (!g) return;
    raf = requestAnimationFrame(tick);
    const now = performance.now();
    if (now - last < cooldown) return;
    const lx = g.axes[0] ?? 0;
    const ly = g.axes[1] ?? 0;
    if (g.buttons[12]?.pressed || ly < -dead) { move(0, -1); last = now; }
    else if (g.buttons[13]?.pressed || ly > dead) { move(0, 1); last = now; }
    else if (g.buttons[14]?.pressed || lx < -dead) { move(-1, 0); last = now; }
    else if (g.buttons[15]?.pressed || lx > dead) { move(1, 0); last = now; }
    else if (g.buttons[0]?.pressed) {
      (document.activeElement as HTMLElement | null)?.click();
      last = now;
    } else if (g.buttons[1]?.pressed) {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
      last = now;
    }
  };

  const wake = () => {
    if (!raf && enabled() && !document.hidden && connected()) raf = requestAnimationFrame(tick);
  };
  window.addEventListener("gamepadconnected", wake);
  document.addEventListener("visibilitychange", wake);
  wake();
  return () => {
    cancelAnimationFrame(raf);
    window.removeEventListener("gamepadconnected", wake);
    document.removeEventListener("visibilitychange", wake);
  };
}

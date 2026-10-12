import { useEffect, useRef, useState } from "react";
import { StoreProvider, useStore, type Route } from "./store";
import { Icon, type IconName } from "./components/Icon";
import { Avatar, InstanceIcon, Progress } from "./components/ui";
import { api } from "./api";
import { Home } from "./pages/Home";
import { Library } from "./pages/Library";
import { InstancePage } from "./pages/Instance";
import { Discover } from "./pages/Discover";
import { Accounts } from "./pages/Accounts";
import { SettingsPage } from "./pages/Settings";
import { CreateInstance } from "./pages/CreateInstance";
import { GlobalSearch } from "./components/GlobalSearch";
import { UpdateBanner } from "./components/UpdateBanner";
import { CairnMark } from "./assets/CairnMark";

const NAV: { name: Route["name"]; label: string; icon: IconName }[] = [
  { name: "home", label: "Home", icon: "home" },
  { name: "library", label: "Library", icon: "library" },
  { name: "discover", label: "Discover", icon: "discover" },
  { name: "accounts", label: "Accounts", icon: "user" },
  { name: "settings", label: "Settings", icon: "settings" },
];

function Wordmark() {
  return (
    <div className="wordmark">
      <CairnMark className="wordmark-mark" size={32} />
      <span className="wordmark-text">cairn</span>
    </div>
  );
}

function AccountSwitcher() {
  const { accounts, setAccounts, go, toast } = useStore();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const active = accounts.find((a) => a.active);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => { if (!ref.current?.contains(e.target as Node)) setOpen(false); };
    const esc = (e: KeyboardEvent) => { if (e.key === "Escape") setOpen(false); };
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", esc);
    return () => { document.removeEventListener("mousedown", close); document.removeEventListener("keydown", esc); };
  }, [open]);
  return (
    <div ref={ref} style={{ position: "relative" }}>
      <button className="switcher" aria-haspopup="menu" aria-expanded={open} onClick={() => (accounts.length ? setOpen(!open) : go({ name: "accounts" }))}>
        <Avatar account={active} small />
        <span style={{ minWidth: 0, flex: 1 }}>
          <span className="switcher-name" style={{ display: "block" }}>{active ? active.username : "No account"}</span>
          <span className="small muted">{active ? (active.kind === "microsoft" ? "Microsoft" : "Offline") : "Add an account"}</span>
        </span>
        <Icon name="chevron" size={14} />
      </button>
      {open && (
        <div className="popover" role="menu" style={{ bottom: "calc(100% + 6px)", left: 0, right: 0 }}>
          {accounts.map((a) => (
            <button key={a.id} role="menuitemradio" aria-checked={a.active} className="menu-item" onClick={async () => {
              try { setAccounts(await api.setActiveAccount(a.id)); setOpen(false); toast(`Playing as ${a.username}`); }
              catch (e) { toast(String(e), true); }
            }}>
              <Avatar account={a} small />
              <span style={{ flex: 1, minWidth: 0 }}>
                <span className="switcher-name" style={{ display: "block" }}>{a.username}</span>
                <span className="small muted">{a.kind === "microsoft" ? "Microsoft" : "Offline"}</span>
              </span>
              {a.active && <span style={{ color: "var(--moss)" }}><Icon name="check" label="Active" /></span>}
            </button>
          ))}
          <hr className="divider" style={{ margin: "6px 0" }} />
          <button role="menuitem" className="menu-item" onClick={() => { setOpen(false); go({ name: "accounts" }); }}>
            <Icon name="user" /> Manage accounts
          </button>
        </div>
      )}
    </div>
  );
}

function Rail() {
  const { route, go, instances, games, setShowSearch } = useStore();
  const recent = (instances ?? []).slice(0, 5);
  const current = route.name === "instance" ? "library" : route.name;
  return (
    <nav className="rail" aria-label="Main">
      <Wordmark />
      <div className="nav">
        {NAV.map((n) => (
          <button key={n.name} className="nav-item" aria-current={current === n.name ? "page" : undefined} onClick={() => go({ name: n.name } as Route)}>
            <Icon name={n.icon} size={20} />
            <span>{n.label}</span>
            {n.name === "library" && instances && instances.length > 0 && <span className="nav-badge">{instances.length}</span>}
          </button>
        ))}
        <button className="nav-item" onClick={() => setShowSearch(true)} title="Search library (Ctrl+K)">
          <Icon name="search" size={20} />
          <span>Search</span>
        </button>
      </div>
      {recent.length > 0 && <div className="rail-section">Recent</div>}
      <div className="rail-recent">
        {recent.map((i) => (
          <button key={i.id} className="nav-item" aria-current={route.name === "instance" && route.id === i.id ? "page" : undefined} onClick={() => go({ name: "instance", id: i.id })}>
            <InstanceIcon name={i.name} src={i.iconPath} small />
            <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>{i.name}</span>
            {games[i.id] && games[i.id].state !== "exited" && <span className="chip chip-moss" style={{ height: 20 }}>{games[i.id].state === "running" ? "Running" : "Starting"}</span>}
          </button>
        ))}
      </div>
      <div className="rail-bottom">
        <AccountSwitcher />
      </div>
    </nav>
  );
}

function TaskTray() {
  const { tasks, dismissTask } = useStore();
  if (!tasks.length) return null;
  return (
    <div className="tray" aria-live="polite">
      {tasks.slice(-3).map((t) => (
        <div key={t.id} className={`task ${t.state === "error" ? "error" : ""}`}>
          <div className="task-title">
            <span style={{ flex: 1, minWidth: 0 }}>{t.title}</span>
            {t.state !== "running" && (
              <button className="btn btn-quiet btn-icon btn-sm" aria-label="Dismiss" onClick={() => dismissTask(t.id)}><Icon name="close" size={14} /></button>
            )}
          </div>
          <div className="task-stage">
            {t.state === "error" ? t.error : t.state === "done" ? "Done" : t.total > 0 ? `${t.stage} (${t.done} of ${t.total})` : t.stage}
          </div>
          {t.state === "running" && <Progress done={t.done} total={t.total} />}
        </div>
      ))}
    </div>
  );
}

function Toasts() {
  const { toasts } = useStore();
  return (
    <div className="toast-region" role="status" aria-live="polite">
      {toasts.map((t) => <div key={t.id} className={`toast ${t.error ? "error" : ""}`}>{t.text}</div>)}
    </div>
  );
}

function Pages() {
  const { route, showCreate, setShowCreate, showSearch, setShowSearch } = useStore();
  const key = JSON.stringify(route);
  return (
    <main className="main">
      <UpdateBanner />
      <div className="page" key={key}>
        <div className="page-inner page-enter">
          {route.name === "home" && <Home />}
          {route.name === "library" && <Library />}
          {route.name === "instance" && <InstancePage id={route.id} initialTab={route.tab} />}
          {route.name === "discover" && <Discover instanceId={route.instanceId} initialType={route.projectType} openProject={route.projectId} />}
          {route.name === "accounts" && <Accounts />}
          {route.name === "settings" && <SettingsPage />}
        </div>
      </div>
      {showCreate && <CreateInstance onClose={() => setShowCreate(false)} />}
      {showSearch && <GlobalSearch onClose={() => setShowSearch(false)} />}
    </main>
  );
}

export function App() {
  return (
    <StoreProvider>
      <div className="app">
        <Rail />
        <Pages />
      </div>
      <TaskTray />
      <Toasts />
    </StoreProvider>
  );
}

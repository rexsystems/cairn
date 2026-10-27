import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { api, on, type Account, type AppInfo, type GameState, type InstanceView, type LaunchOpts, type Settings, type TaskEvent } from "./api";
import { startGamepadNav } from "./gamepad";

export type Route =
  | { name: "home" }
  | { name: "library" }
  | { name: "instance"; id: string; tab?: string }
  | { name: "discover"; instanceId?: string; projectType?: string; projectId?: string }
  | { name: "accounts" }
  | { name: "settings" };

interface Toast { id: number; text: string; error?: boolean }

interface Store {
  route: Route;
  go: (r: Route) => void;
  back: () => void;
  instances: InstanceView[] | null;
  instancesError: string | null;
  refreshInstances: () => Promise<void>;
  accounts: Account[];
  refreshAccounts: () => Promise<void>;
  setAccounts: (a: Account[]) => void;
  settings: Settings | null;
  setSettings: (s: Settings) => void;
  info: AppInfo | null;
  games: Record<string, GameState>;
  tasks: TaskEvent[];
  dismissTask: (id: string) => void;
  toast: (text: string, error?: boolean) => void;
  toasts: Toast[];
  play: (id: string, opts?: LaunchOpts | string) => Promise<void>;
  showCreate: boolean;
  setShowCreate: (v: boolean) => void;
  showSearch: boolean;
  setShowSearch: (v: boolean) => void;
}

const Ctx = createContext<Store>(null as unknown as Store);
export const useStore = () => useContext(Ctx);

export function StoreProvider({ children }: { children: ReactNode }) {
  const [history, setHistory] = useState<Route[]>([{ name: "home" }]);
  const route = history[history.length - 1];
  const go = useCallback((r: Route) => setHistory((h) => [...h.slice(-30), r]), []);
  const back = useCallback(() => setHistory((h) => (h.length > 1 ? h.slice(0, -1) : h)), []);

  const [instances, setInstances] = useState<InstanceView[] | null>(null);
  const [instancesError, setInstancesError] = useState<string | null>(null);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [games, setGames] = useState<Record<string, GameState>>({});
  const [tasks, setTasks] = useState<TaskEvent[]>([]);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [showSearch, setShowSearch] = useState(false);
  const toastId = useRef(0);
  const instancesRef = useRef<InstanceView[] | null>(null);
  const gamesRef = useRef<Record<string, GameState>>({});

  const toast = useCallback((text: string, error?: boolean) => {
    const id = ++toastId.current;
    setToasts((t) => [...t.slice(-3), { id, text, error }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), error ? 7000 : 3800);
  }, []);

  const refreshInstances = useCallback(async () => {
    try {
      const list = await api.listInstances();
      instancesRef.current = list;
      setInstances(list);
      setInstancesError(null);
    } catch (e) {
      setInstancesError(String(e));
    }
  }, []);
  const refreshAccounts = useCallback(async () => setAccounts(await api.listAccounts()), []);

  useEffect(() => {
    refreshInstances();
    refreshAccounts();
    api.getSettings().then(setSettings);
    api.appInfo().then(setInfo);
    api.gameStates().then((s) => {
      const map = Object.fromEntries(s.map((g) => [g.instanceId, g]));
      gamesRef.current = map;
      setGames(map);
    });
    const subs = [
      on<GameState>("game-state", (g) => {
        const prev = gamesRef.current[g.instanceId];
        const next = { ...gamesRef.current, [g.instanceId]: g };
        gamesRef.current = next;
        setGames(next);
        if (g.state === "exited") {
          refreshInstances();
          const wasLive = prev && prev.state !== "exited";
          const crashed = wasLive && g.exitCode != null && g.exitCode !== 0;
          if (crashed) {
            const name = instancesRef.current?.find((i) => i.id === g.instanceId)?.name ?? "Instance";
            toast(`${name} crashed (exit ${g.exitCode}). Open Logs on that instance for the report.`, true);
          }
        }
      }),
      on<TaskEvent>("task", (t) => {
        setTasks((list) => {
          const others = list.filter((x) => x.id !== t.id);
          return [...others, t];
        });
        if (t.state === "done") setTimeout(() => setTasks((l) => l.filter((x) => x.id !== t.id)), 2500);
      }),
    ];
    return () => { subs.forEach((p) => p.then((u) => u())); };
  }, [refreshInstances, refreshAccounts, toast]);

  // Pick up changes made outside the app (copied folders, renamed instances) when the user moves around.
  useEffect(() => {
    if (route.name === "home" || route.name === "library") refreshInstances();
  }, [route, refreshInstances]);
  useEffect(() => {
    const f = () => refreshInstances();
    window.addEventListener("focus", f);
    return () => window.removeEventListener("focus", f);
  }, [refreshInstances]);

  useEffect(() => {
    if (!settings) return;
    const apply = () => {
      const dark = settings.theme === "dark" || (settings.theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
      document.documentElement.dataset.theme = dark ? "dark" : "light";
      document.documentElement.dataset.motion = settings.reduceMotion ? "reduced" : "full";
    };
    apply();
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [settings]);

  const play = useCallback(async (id: string, opts?: LaunchOpts | string) => {
    try {
      await api.launch(id, opts);
    } catch (e) {
      toast(String(e), true);
    }
  }, [toast]);

  useEffect(() => {
    return startGamepadNav(() => !!settings?.gamepadNav);
  }, [settings?.gamepadNav]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setShowSearch(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const dismissTask = useCallback((id: string) => setTasks((l) => l.filter((x) => x.id !== id)), []);

  const value = useMemo<Store>(() => ({
    route, go, back, instances, instancesError, refreshInstances, accounts, refreshAccounts, setAccounts,
    settings, setSettings, info, games, tasks, dismissTask, toast, toasts, play, showCreate, setShowCreate,
    showSearch, setShowSearch,
  }), [route, go, back, instances, instancesError, refreshInstances, accounts, refreshAccounts, settings, info, games, tasks, dismissTask, toast, toasts, play, showCreate, showSearch]);

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

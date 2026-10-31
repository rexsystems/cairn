import { useEffect, useMemo, useRef, useState } from "react";
import { api, on, type FileEntry, type InstanceView, type LogLine } from "../../api";
import { useStore } from "../../store";
import { Icon } from "../../components/Icon";
import { Select } from "../../components/ui";
import { classify } from "../../lib/logs";

export function LogsTab({ inst }: { inst: InstanceView }) {
  const { games, toast } = useStore();
  const [source, setSource] = useState<string>("live");
  const [lines, setLines] = useState<{ t: string; s: string }[]>([]);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [fileText, setFileText] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [follow, setFollow] = useState(true);
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.gameLog(inst.id).then((l) => setLines(l.map((t) => ({ t, s: "stdout" }))));
    api.listLogFiles(inst.id).then(setFiles).catch(() => setFiles([]));
    const un = on<LogLine>("game-log", (l) => {
      if (l.instanceId === inst.id) setLines((prev) => (prev.length > 8000 ? [...prev.slice(-6000), { t: l.line, s: l.stream }] : [...prev, { t: l.line, s: l.stream }]));
    });
    return () => { un.then((u) => u()); };
  }, [inst.id]);

  useEffect(() => {
    if (source === "live") { setFileText(null); return; }
    api.readInstanceFile(inst.id, source).then(setFileText).catch((e) => setFileText(String(e)));
  }, [source, inst.id]);

  const shown = useMemo(() => {
    const base = source === "live" ? lines : (fileText ?? "").split("\n").map((t) => ({ t, s: "stdout" }));
    const f = filter.trim().toLowerCase();
    return f ? base.filter((l) => l.t.toLowerCase().includes(f)) : base;
  }, [lines, fileText, source, filter]);

  useEffect(() => { if (follow && box.current) box.current.scrollTop = box.current.scrollHeight; }, [shown, follow]);

  const g = games[inst.id];
  return (
    <div className="stack">
      <div className="row" style={{ gap: 12 }}>
        <div className="field" style={{ flex: "0 1 320px" }}>
          <span className="sr-only">Log source</span>
          <Select
            aria-label="Log source"
            value={source}
            onChange={setSource}
            options={[
              { value: "live", label: `Current session${g && g.state !== "exited" ? " (running)" : ""}` },
              ...files.map((f) => ({ value: f.path, label: f.name })),
            ]}
          />
        </div>
        <label className="search" style={{ maxWidth: 320 }}>
          <span className="sr-only">Filter log lines</span>
          <Icon name="search" size={16} />
          <input className="input" placeholder="Filter lines" value={filter} onChange={(e) => setFilter(e.target.value)} />
        </label>
        <label className="check"><input type="checkbox" checked={follow} onChange={(e) => setFollow(e.target.checked)} />Follow output</label>
        <span className="spacer" />
        <button className="btn" onClick={() => navigator.clipboard.writeText(shown.map((l) => l.t).join("\n")).then(() => toast("Copied log to clipboard"))}><Icon name="copy" />Copy</button>
        {source !== "live" && (
          <button className="btn" onClick={() => {
            const folder = source.includes("crash-reports") ? `${inst.path}/crash-reports` : source.includes("/logs/") ? `${inst.path}/logs` : `${inst.path}/.cairn/logs`;
            api.openDataPath(folder).catch((e) => toast(String(e), true));
          }}><Icon name="folder" />Open folder</button>
        )}
      </div>
      {files.some((f) => f.name.startsWith("Crash report:")) && (
        <div className="notice" role="status">
          <Icon name="warn" />
          <div style={{ flex: 1 }}>
            <b>Crash reports on disk</b>
            <p className="small muted">Pick a “Crash report” in the source list, then copy it or open the folder.</p>
          </div>
          <button className="btn btn-sm" onClick={() => {
            const crash = files.find((f) => f.name.startsWith("Crash report:"));
            if (crash) setSource(crash.path);
          }}>Show latest</button>
          <button className="btn btn-sm" onClick={() => api.openDataPath(`${inst.path}/crash-reports`).catch((e) => toast(String(e), true))}><Icon name="folder" size={14} />Open crashes</button>
        </div>
      )}
      <div className="log-view" ref={box} tabIndex={0} aria-label="Game log" role="log">
        {shown.length === 0 ? (
          <span className="muted">{source === "live" ? "Nothing logged yet. Press Play and the game output streams here." : fileText === null ? "Reading file..." : "This file is empty."}</span>
        ) : shown.map((l, i) => <div key={i} className={`log-line ${classify(l.t, l.s)}`}>{l.t}</div>)}
      </div>
    </div>
  );
}

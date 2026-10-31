import { useEffect, useState } from "react";
import { api, type CrashAnalysis } from "../api";
import { Icon } from "./Icon";
import { CompatIssues } from "./CompatIssues";

/** Reads a log or crash report and lists the causes Cairn recognises, with their fixes. */
export function CrashHelp({ instanceId, log, onClose }: { instanceId: string; log?: string; onClose: () => void }) {
  const [result, setResult] = useState<CrashAnalysis | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [round, setRound] = useState(0);

  useEffect(() => {
    let live = true;
    setResult(null);
    setError(null);
    api.analyzeCrash(instanceId, log).then((r) => live && setResult(r)).catch((e) => live && setError(String(e)));
    return () => { live = false; };
  }, [instanceId, log, round]);

  const close = <button className="btn btn-sm" onClick={onClose} aria-label="Close"><Icon name="close" size={14} /></button>;
  if (error) return <div className="notice error" role="alert"><Icon name="warn" /><div style={{ flex: 1 }}>{error}</div>{close}</div>;
  if (!result) return <div className="notice" role="status"><Icon name="search" /><div style={{ flex: 1 }}>Reading the log...</div></div>;
  if (!result.source) return <div className="notice" role="status"><Icon name="check" /><div style={{ flex: 1 }}>No crash or session log yet.</div>{close}</div>;
  if (result.issues.length === 0) {
    return (
      <div className="notice" role="status">
        <Icon name="log" />
        <div style={{ flex: 1 }}>
          <b>No known cause in {result.source}</b>
          <p className="small muted">Look for the first “Caused by” line, or share the log when asking for help.</p>
        </div>
        {close}
      </div>
    );
  }
  return (
    <div className="stack" style={{ gap: 8 }}>
      <div className="row" style={{ gap: 8 }}>
        <b>What went wrong</b>
        <span className="small muted">from {result.source}</span>
        <span className="spacer" />
        {close}
      </div>
      <CompatIssues instanceId={instanceId} issues={result.issues} onFixed={() => setRound((n) => n + 1)} />
    </div>
  );
}

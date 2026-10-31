import { useState } from "react";
import { api, type CompatIssue, type FixAction } from "../api";
import { useStore } from "../store";
import { runsInBackend } from "../lib/install";
import { Icon } from "./Icon";
import { ManualDownloadsDialog } from "./ManualDownloads";

/** Issue list with one-click fixes, shared by the instance page and the pre-launch check. */
export function CompatIssues({ instanceId, issues, onFixed }: { instanceId: string; issues: CompatIssue[]; onFixed: () => void }) {
  const { toast, go } = useStore();
  const [busy, setBusy] = useState<string | null>(null);
  const [manual, setManual] = useState(false);

  const run = async (issue: CompatIssue, f: FixAction) => {
    const { label: _label, ...fix } = f;
    if (fix.type === "searchMod") { go({ name: "discover", instanceId, projectType: "mod", query: fix.query }); return; }
    if (fix.type === "manualDownloads") { setManual(true); return; }
    if (!runsInBackend(fix)) return;
    setBusy(issue.key);
    try {
      const msg = await api.applyFix(instanceId, fix);
      if (msg) toast(msg);
      onFixed();
    } catch (e) { toast(String(e), true); } finally { setBusy(null); }
  };

  return (
    <>
      <div className="stack" style={{ gap: 8 }}>
        {issues.map((i) => (
          <div key={i.key} className={`notice ${i.severity === "error" ? "error" : "warn"}`}>
            <Icon name="warn" />
            <div style={{ flex: 1, minWidth: 0 }}>
              <b>{i.title}</b>
              <p className="small muted">{i.detail}</p>
              {i.files.length > 0 && <p className="small muted">{i.files.slice(0, 4).join(", ")}{i.files.length > 4 ? ` and ${i.files.length - 4} more` : ""}</p>}
            </div>
            <div className="row" style={{ gap: 6, justifyContent: "flex-end" }}>
              {i.fixes.map((f, n) => (
                <button key={n} className={`btn btn-sm ${n === 0 ? "btn-play" : ""}`} disabled={busy !== null} onClick={() => run(i, f)}>
                  {busy === i.key && n === 0 ? "Working..." : f.label}
                </button>
              ))}
            </div>
          </div>
        ))}
      </div>
      {manual && <ManualDownloadsDialog instanceId={instanceId} onClose={() => { setManual(false); onFixed(); }} />}
    </>
  );
}

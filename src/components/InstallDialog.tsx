import { useEffect, useState } from "react";
import { api, type ContentSource, type InstallPlan, type InstallRoot, type PlanItem } from "../api";
import { useStore } from "../store";
import { installedMessage, isManual, summarizePlan } from "../lib/install";
import { Icon } from "./Icon";
import { ErrorState, Loading, Modal } from "./ui";

export interface InstallRequest {
  instanceId: string;
  source: ContentSource;
  roots: InstallRoot[];
  title: string;
  includeOptional?: boolean;
  onDone?: () => void;
}

/** Resolves the plan first and installs straight away when it holds no surprises. */
export function InstallDialog({ req, onClose }: { req: InstallRequest; onClose: () => void }) {
  const { instances, toast, refreshInstances } = useStore();
  const inst = instances?.find((i) => i.id === req.instanceId);
  const name = inst?.name ?? "the instance";
  const [plan, setPlan] = useState<InstallPlan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [optional, setOptional] = useState(!!req.includeOptional);
  const [attempt, setAttempt] = useState(0);

  const install = async (allowProblems: boolean) => {
    setBusy(true);
    try {
      const r = await api.installContent(req.instanceId, req.source, req.roots, optional, allowProblems);
      toast(installedMessage(r.installed, r.pending.length, name), r.pending.length > 0 && r.installed.length === 0);
      req.onDone?.();
      refreshInstances();
      onClose();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  useEffect(() => {
    let alive = true;
    setPlan(null);
    setError(null);
    api.planInstall(req.instanceId, req.source, req.roots, optional)
      .then((p) => {
        if (!alive) return;
        setPlan(p);
        if (!summarizePlan(p).needsReview && !optional) void install(false);
      })
      .catch((e) => alive && setError(String(e)));
    return () => { alive = false; };
  }, [req, optional, attempt]);

  const s = plan && summarizePlan(plan);
  const ready = plan && plan.items.length > 0;
  const footer = plan && !busy ? (
    <>
      <button className="btn" onClick={onClose}>Cancel</button>
      {ready && (
        <button className="btn btn-play" onClick={() => install(plan.problems.length > 0)}>
          <Icon name="download" />
          {plan.problems.length ? "Install anyway" : `Install ${plan.items.length} ${plan.items.length === 1 ? "file" : "files"}`}
        </button>
      )}
    </>
  ) : undefined;

  return (
    <Modal title={`Install ${req.title}`} onClose={busy ? () => {} : onClose} footer={footer}>
      {error && <ErrorState title="Could not install" error={error} onRetry={() => setAttempt((a) => a + 1)} />}
      {!error && (!plan || busy) && <Loading label={busy ? `Installing into ${name}` : "Checking dependencies"} />}
      {!error && plan && s && !busy && (
        <div className="stack" style={{ gap: 14 }}>
          {plan.problems.length > 0 && (
            <div className="notice error" role="alert">
              <Icon name="warn" />
              <div className="stack" style={{ gap: 4 }}>
                <b>This may not work in {name}</b>
                {plan.problems.map((p, i) => <span key={i} className="small">{p.title}: {p.message}</span>)}
              </div>
            </div>
          )}
          {plan.items.length === 0 && <p className="muted">Nothing to install. {name} already has everything this needs.</p>}
          {s.requested.length > 0 && <PlanGroup title="Selected" items={s.requested} />}
          {s.dependencies.length > 0 && <PlanGroup title="Required dependencies" items={s.dependencies} />}
          {s.manual.length > 0 && (
            <p className="small muted">
              {s.manual.length === 1 ? "One file" : `${s.manual.length} files`} can only be downloaded on the CurseForge website.
              After installing, Cairn opens the page and picks the file up from your Downloads folder.
            </p>
          )}
          {plan.skipped.length > 0 && (
            <details>
              <summary className="small muted">{plan.skipped.length} already installed or skipped</summary>
              <ul className="small muted">{plan.skipped.map((n, i) => <li key={i}>{n.title}: {n.message}</li>)}</ul>
            </details>
          )}
          <label className="check">
            <input type="checkbox" checked={optional} onChange={(e) => setOptional(e.target.checked)} />
            Include optional dependencies
          </label>
        </div>
      )}
    </Modal>
  );
}

function PlanGroup({ title, items }: { title: string; items: PlanItem[] }) {
  return (
    <div className="stack" style={{ gap: 6 }}>
      <h3 className="small muted" style={{ textTransform: "uppercase", letterSpacing: ".04em" }}>{title}</h3>
      <div className="card content-list">
        {items.map((it) => (
          <div key={`${it.candidate.project.id}-${it.candidate.versionId}`} className="content-row">
            {it.candidate.project.icon ? <img className="content-icon" src={it.candidate.project.icon} alt="" /> : <div className="content-icon" aria-hidden="true" />}
            <div className="content-main">
              <div className="content-title">
                {it.candidate.project.title}
                {isManual(it) && <span className="chip chip-ember" style={{ marginLeft: 8 }}>Manual download</span>}
                {it.replaces.length > 0 && <span className="chip" style={{ marginLeft: 8 }}>Replaces {it.replaces.length}</span>}
              </div>
              <div className="content-sub">
                {[it.candidate.versionNumber, it.reason.type === "dependencyOf" ? `needed by ${it.reason.title}` : null, it.optional ? "optional" : null].filter(Boolean).join("  ·  ")}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

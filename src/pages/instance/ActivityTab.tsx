import { useEffect, useState } from "react";
import { api, type InstanceStats, type InstanceView, type PlaySession } from "../../api";
import { playTime } from "../../format";
import { Icon } from "../../components/Icon";
import { CrashHelp } from "../../components/CrashHelp";
import { useStore } from "../../store";

const DAY_NAMES = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

function dayLabel(daysAgo: number): string {
  if (daysAgo === 0) return "Today";
  const d = new Date();
  d.setDate(d.getDate() - daysAgo);
  return `${DAY_NAMES[d.getDay()]} ${d.getDate()}`;
}

function when(ms: number): string {
  return new Date(ms).toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

/** Playtime, recent sessions and the crashes among them. */
export function ActivityTab({ inst }: { inst: InstanceView }) {
  const { games } = useStore();
  const [stats, setStats] = useState<InstanceStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState<PlaySession | null>(null);
  const state = games[inst.id]?.state;

  useEffect(() => {
    api.instanceStats(inst.id).then(setStats).catch((e) => setError(String(e)));
  }, [inst.id, state]);

  if (error) return <div className="notice error" role="alert"><Icon name="warn" />{error}</div>;
  if (!stats) return <p className="muted">Loading...</p>;
  const peak = Math.max(...stats.days, 1);

  return (
    <div className="stack">
      <div className="activity-tiles">
        <div className="card activity-tile"><span className="small muted">Played</span><b>{playTime(stats.totalSeconds)}</b></div>
        <div className="card activity-tile"><span className="small muted">Last 7 days</span><b>{playTime(stats.weekSeconds)}</b></div>
        <div className="card activity-tile"><span className="small muted">Sessions</span><b>{stats.sessions}</b></div>
        <div className="card activity-tile"><span className="small muted">Longest</span><b>{playTime(stats.longestSeconds)}</b></div>
        <div className="card activity-tile"><span className="small muted">Crashes</span><b>{stats.crashes}</b></div>
      </div>

      <section className="card activity-chart" aria-label="Playtime in the last 14 days">
        {stats.days.map((s, i) => {
          const ago = stats.days.length - 1 - i;
          return (
            <div key={i} className="activity-day" title={`${dayLabel(ago)}: ${playTime(s)}`}>
              <div className="activity-bar" style={{ height: `${Math.round((s / peak) * 100)}%` }} />
              <span className="small muted">{ago % 2 === 0 ? dayLabel(ago) : ""}</span>
            </div>
          );
        })}
      </section>

      {open && <CrashHelp instanceId={inst.id} log={open.log ?? undefined} onClose={() => setOpen(null)} />}

      <section className="stack" style={{ gap: 6 }}>
        <h3>Recent sessions</h3>
        {stats.recent.length === 0 ? (
          <p className="muted">Sessions show up here after you play.</p>
        ) : (
          <ul className="activity-list">
            {stats.recent.map((s) => (
              <li key={s.started} className={s.crashed ? "crashed" : ""}>
                <span>{when(s.started)}</span>
                <span className="muted">{playTime(s.seconds)}</span>
                <span className="activity-result">
                  {s.crashed ? <><Icon name="warn" size={14} />{s.reason ?? `Crashed${s.exitCode != null ? ` (exit ${s.exitCode})` : ""}`}</> : "Closed normally"}
                </span>
                {s.crashed && s.log && <button className="btn btn-sm" onClick={() => setOpen(s)}>Find the cause</button>}
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

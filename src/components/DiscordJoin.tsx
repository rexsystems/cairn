import { useCallback, useEffect, useState } from "react";
import { api, on, type DiscordEvent, type JoinPlan } from "../api";
import { useStore } from "../store";
import { Icon } from "./Icon";
import { Modal } from "./ui";
import { describeJoin, joinNeedsDownload, runJoin, type JoinSource } from "../lib/join";

type Request = Extract<DiscordEvent, { kind: "joinRequest" }>;

/** Acts on Discord: Join clicks start the matching instance, ask-to-join requests get Accept and Decline. */
export function DiscordJoin() {
  const { play, toast, refreshInstances, go } = useStore();
  const [pending, setPending] = useState<{ plan: JoinPlan; from: JoinSource } | null>(null);
  const [requests, setRequests] = useState<Request[]>([]);
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);

  const join = useCallback(async (plan: JoinPlan, from: JoinSource) => {
    setBusy(true);
    try {
      const id = await runJoin(plan, from, play);
      await refreshInstances();
      go({ name: "instance", id });
      setPending(null);
    } catch (e) {
      toast(String(e), true);
    } finally {
      setBusy(false);
    }
  }, [play, refreshInstances, go, toast]);

  const plan = useCallback(async (from: JoinSource) => {
    try {
      const p = await api.joinPlan(from);
      // A matching instance needs no download, so Join from Discord goes straight in.
      if (p.step.kind === "launch") await join(p, from);
      else setPending({ plan: p, from });
    } catch (e) {
      toast(`Couldn't join: ${String(e)}`, true);
    }
  }, [join, toast]);

  useEffect(() => {
    // The pending slot is taken by whoever reads it first, so a join is acted on once.
    const takeJoin = () => api.takeDiscordJoin().then((secret) => { if (secret) plan({ secret }); }).catch(() => {});
    takeJoin();
    const sub = on<DiscordEvent>("discord-event", (ev) => {
      if (ev.kind === "join") takeJoin();
      else setRequests((r) => [...r.filter((x) => x.userId !== ev.userId), ev].slice(-3));
    });
    return () => { sub.then((u) => u()); };
  }, [plan]);

  const reply = async (r: Request, accept: boolean) => {
    setRequests((all) => all.filter((x) => x.userId !== r.userId));
    try {
      await api.discordReply(r.userId, accept, r.username, r.avatarUrl);
      if (accept) toast(`${r.username} can join now`);
    } catch (e) {
      toast(String(e), true);
    }
  };

  const useCode = async () => {
    try {
      const p = await api.joinPlan({ code: code.trim() });
      setPending({ plan: p, from: { code: code.trim() } });
      setCode("");
    } catch (e) {
      toast(String(e), true);
    }
  };

  return (
    <>
      {requests.length > 0 && (
        <div className="join-requests" role="region" aria-label="Ask to join requests">
          {requests.map((r) => (
            <div key={r.userId} className="join-request card">
              {r.avatarUrl ? <img src={r.avatarUrl} alt="" width={32} height={32} className="join-avatar" /> : <Icon name="friends" size={24} />}
              <span className="join-request-text"><b>{r.username}</b> wants to join you</span>
              <button className="btn btn-sm" onClick={() => reply(r, true)}><Icon name="check" size={14} />Accept</button>
              <button className="btn btn-quiet btn-sm" onClick={() => reply(r, false)}>Decline</button>
            </div>
          ))}
        </div>
      )}
      {pending && (
        <Modal title="Join from Discord" onClose={() => !busy && setPending(null)} footer={
          <>
            <button className="btn btn-quiet" onClick={() => setPending(null)} disabled={busy}>Cancel</button>
            {pending.plan.step.kind === "needCode" ? (
              <button className="btn" onClick={useCode} disabled={!code.trim()}>Use code</button>
            ) : (
              <button className="btn btn-play" onClick={() => join(pending.plan, pending.from)} disabled={busy} data-autofocus>
                <Icon name={joinNeedsDownload(pending.plan) ? "download" : "play"} />{busy ? "Setting up..." : "Set up and join"}
              </button>
            )}
          </>
        }>
          <p>{describeJoin(pending.plan)}</p>
          {pending.plan.step.kind === "needCode" && (
            <label className="field">
              <span>Invite code</span>
              <textarea className="input mono share-code" value={code} onChange={(e) => setCode(e.target.value)} placeholder="CAIRN1:...@server" data-autofocus />
            </label>
          )}
          <p className="small muted">Server: <code className="mono">{pending.plan.invite.address}</code></p>
        </Modal>
      )}
    </>
  );
}

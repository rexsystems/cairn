import { useEffect, useState } from "react";
import { api, type DiscordStatus, type Friend } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { EmptyState, ErrorState, Loading } from "../components/ui";
import { InviteDialog } from "../components/InviteDialog";
import { CairnFriends } from "../components/CairnFriends";
import { relativeTime } from "../format";
import { describeJoin, runJoin } from "../lib/join";

/** Cairn Client friends from the Cairn server, then people you played with through invites (kept on this computer). */
export function Friends() {
  const { toast, play, refreshInstances, go, instances, games } = useStore();
  const [friends, setFriends] = useState<Friend[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [discord, setDiscord] = useState<DiscordStatus | null>(null);
  const [inviting, setInviting] = useState<{ id: string; name: string } | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const load = () => api.friendsList().then(setFriends).catch((e) => setError(String(e)));
  useEffect(() => {
    load();
    api.discordStatus().then(setDiscord).catch(() => {});
  }, []);

  const joinAgain = async (f: Friend) => {
    const from = f.secret ? { secret: f.secret } : { code: f.code ?? "" };
    setBusy(f.id);
    try {
      const plan = await api.joinPlan(from);
      if (plan.step.kind === "needCode") throw new Error(describeJoin(plan));
      const id = await runJoin(plan, from, play);
      await refreshInstances();
      go({ name: "instance", id });
    } catch (e) {
      toast(String(e), true);
    } finally {
      setBusy(null);
    }
  };

  const reinvite = (f: Friend) => {
    const running = games[f.instanceId] && games[f.instanceId].state !== "exited";
    if (!instances?.some((i) => i.id === f.instanceId)) { toast("That instance was deleted.", true); return; }
    if (!running) toast(`Start ${f.instanceName} or its server first, then invite.`);
    setInviting({ id: f.instanceId, name: f.instanceName });
  };

  const forget = async (f: Friend) => {
    try { setFriends(await api.friendsForget(f.id)); } catch (e) { toast(String(e), true); }
  };

  if (error) return <ErrorState title="Couldn't load your friends" error={error} onRetry={load} />;
  if (!friends) return <Loading label="Loading friends" />;
  return (
    <div className="stack">
      <div className="page-head">
        <div>
          <h1>Friends</h1>
          <p className="muted">Friends who play with Cairn Client, and people you played with through Cairn invites.</p>
        </div>
      </div>
      <CairnFriends />
      <h2>Invites and Discord</h2>
      <div className="notice" role="note">
        <Icon name="warn" />
        <p className="small">
          Discord doesn't let apps like Cairn read your friends list or who's online, so this list only has people you played with through an invite.
          To invite someone new, use Invite friends in an instance or the Host tab, then send it from Discord or copy the invite code.
          {discord && !discord.connected && " Discord isn't connected right now; open the Discord desktop app and turn on Discord status in Settings."}
        </p>
      </div>
      {friends.length === 0 ? (
        <EmptyState title="No one yet" icon="friends">When a friend joins you, or you join a friend, they show up here.</EmptyState>
      ) : (
        <ul className="friend-list">
          {friends.map((f) => (
            <li key={f.id} className="card friend">
              {f.avatarUrl ? <img src={f.avatarUrl} alt="" width={36} height={36} className="join-avatar" /> : <Icon name="friends" size={28} />}
              <div className="friend-text">
                <b>{f.name}</b>
                <span className="small muted">{f.kind === "joinedYou" ? `Joined your ${f.instanceName}` : "You joined them"} {relativeTime(f.at)}</span>
              </div>
              {f.kind === "joinedYou" ? (
                <button className="btn btn-sm" onClick={() => reinvite(f)}><Icon name="friends" size={14} />Invite again</button>
              ) : (
                <button className="btn btn-sm" disabled={busy === f.id} onClick={() => joinAgain(f)}><Icon name="play" size={14} />{busy === f.id ? "Joining..." : "Join again"}</button>
              )}
              <button className="btn btn-quiet btn-icon btn-sm" aria-label={`Forget ${f.name}`} title="Forget" onClick={() => forget(f)}><Icon name="close" size={14} /></button>
            </li>
          ))}
        </ul>
      )}
      {inviting && <InviteDialog id={inviting.id} name={inviting.name} onClose={() => setInviting(null)} />}
    </div>
  );
}

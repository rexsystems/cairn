import { useEffect, useState, type FormEvent } from "react";
import { api, type CairnFriend, type CairnFriendAction, type CairnFriendList, type CairnPerson, type SocialSettings } from "../api";
import { useStore } from "../store";
import { Icon } from "./Icon";
import { Switch } from "./ui";

/** What a Cairn friend is doing. Only the Cairn Client mod reports this, so no mod means offline. */
export function describeActivity(f: CairnFriend): string {
  if (f.status === "offline") return "Offline";
  const dnd = f.status === "dnd" ? "Do not disturb · " : "";
  const a = f.activity;
  if (!a) return `${dnd}Online in Cairn Client`;
  if (a.mode === "multiplayer") return `${dnd}Multiplayer ${a.version}${a.server ? ` on ${a.server}` : ""}`;
  if (a.mode === "singleplayer") return `${dnd}Singleplayer ${a.version}`;
  return `${dnd}In the menus, ${a.version}`;
}

const SHARE: { value: SocialSettings["share"]; label: string }[] = [
  { value: "full", label: "Game and server" },
  { value: "game", label: "Game only" },
  { value: "status", label: "Online only" },
];

/** Friends between Cairn Client players, kept on the cosmetics server. Separate from Discord invites. */
export function CairnFriends() {
  const { toast } = useStore();
  const [list, setList] = useState<CairnFriendList | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [target, setTarget] = useState("");
  const [busy, setBusy] = useState(false);

  const load = () => api.cairnFriends().then((l) => { setList(l); setError(null); }).catch((e) => setError(String(e)));
  useEffect(() => {
    load();
    const timer = window.setInterval(load, 30_000);
    return () => window.clearInterval(timer);
  }, []);

  const act = async (action: CairnFriendAction, who: string, done?: (l: CairnFriendList) => string) => {
    setBusy(true);
    try {
      const l = await api.cairnFriendAction(action, who);
      setList(l);
      if (done) toast(done(l));
    } catch (e) {
      toast(String(e), true);
    } finally {
      setBusy(false);
    }
  };
  const add = (e: FormEvent) => {
    e.preventDefault();
    const who = target.trim();
    if (!who) return;
    act("add", who, (l) => (l.reply?.status === "accepted" ? `You and ${l.reply?.name ?? who} are now friends.` : `Friend request sent to ${l.reply?.name ?? who}.`)).then(() => setTarget(""));
  };
  const setting = async (patch: Partial<SocialSettings>) => {
    if (!list) return;
    setList({ ...list, settings: { ...list.settings, ...patch } });
    try {
      const settings = await api.cairnSocialSettings(patch);
      setList((l) => (l ? { ...l, settings } : l));
    } catch (e) {
      toast(String(e), true);
      load();
    }
  };

  if (error) {
    return (
      <section className="panel cairn-friends" aria-labelledby="cairn-friends">
        <header><h2 id="cairn-friends">Cairn Client friends</h2></header>
        <p className="small muted">{error}</p>
        <p className="small muted">Cairn friends need a Cairn server (Settings) and a Microsoft account that has signed in to it.</p>
        <button className="btn btn-sm" onClick={load}><Icon name="update" size={14} />Try again</button>
      </section>
    );
  }
  if (!list) return <section className="panel cairn-friends"><p className="small muted">Loading Cairn friends...</p></section>;
  const s = list.settings;
  const person = (p: CairnPerson, actions: [CairnFriendAction, string][]) => (
    <li key={p.uuid} className="card friend">
      <Icon name="user" size={22} />
      <div className="friend-text"><b>{p.name || p.uuid}</b></div>
      {actions.map(([action, label]) => (
        <button key={action} className={`btn btn-sm ${action === "accept" ? "" : "btn-quiet"}`} disabled={busy} onClick={() => act(action, p.uuid)}>{label}</button>
      ))}
    </li>
  );
  return (
    <section className="panel cairn-friends stack" aria-labelledby="cairn-friends">
      <header>
        <h2 id="cairn-friends">Cairn Client friends</h2>
        <p className="small muted">Players who use Cairn Client. Their status comes from the mod while they play, so anyone not running Cairn Client shows as offline.</p>
      </header>
      <form className="row" onSubmit={add}>
        <input className="input" aria-label="Minecraft name or UUID" placeholder="Add by Minecraft name or UUID" value={target} maxLength={36} onChange={(e) => setTarget(e.target.value)} />
        <button className="btn" disabled={busy || !target.trim()}><Icon name="plus" size={14} />Add friend</button>
      </form>
      <div className="cairn-friend-settings">
        <Switch checked={s.dnd} onChange={(dnd) => setting({ dnd })} label="Do not disturb (mutes friend toasts in game)" />
        <Switch checked={s.appearOffline} onChange={(appearOffline) => setting({ appearOffline })} label="Appear offline" />
        <Switch checked={s.requests === "everyone"} onChange={(on) => setting({ requests: on ? "everyone" : "none" })} label="Allow friend requests" />
        <div className="field">
          <span>Friends can see</span>
          <div className="segmented" role="group" aria-label="Friends can see">
            {SHARE.map((o) => <button key={o.value} aria-pressed={s.share === o.value} onClick={() => setting({ share: o.value })}>{o.label}</button>)}
          </div>
        </div>
      </div>
      {list.incoming.length > 0 && (
        <>
          <h3>Friend requests</h3>
          <ul className="friend-list">{list.incoming.map((p) => person(p, [["accept", "Accept"], ["decline", "Decline"], ["block", "Block"]]))}</ul>
        </>
      )}
      {list.friends.length === 0 ? (
        <p className="small muted">No Cairn friends yet. Add someone who plays with Cairn Client.</p>
      ) : (
        <ul className="friend-list">
          {list.friends.map((f) => (
            <li key={f.uuid} className="card friend">
              <span className={`presence-dot ${f.status}`} aria-hidden="true" />
              <div className="friend-text">
                <b>{f.name}</b>
                <span className="small muted">{describeActivity(f)}</span>
              </div>
              {f.activity?.server && (
                <button className="btn btn-sm btn-quiet" onClick={() => navigator.clipboard.writeText(f.activity?.server ?? "").then(() => toast("Server address copied."))}><Icon name="copy" size={14} />Copy address</button>
              )}
              <button className="btn btn-sm btn-quiet" disabled={busy} onClick={() => act("remove", f.uuid, () => `Removed ${f.name}.`)}>Remove</button>
              <button className="btn btn-sm btn-quiet" disabled={busy} onClick={() => act("block", f.uuid, () => `Blocked ${f.name}.`)}>Block</button>
            </li>
          ))}
        </ul>
      )}
      {list.outgoing.length > 0 && (
        <>
          <h3>Sent requests</h3>
          <ul className="friend-list">{list.outgoing.map((p) => person(p, [["cancel", "Cancel"]]))}</ul>
        </>
      )}
      {list.blocked.length > 0 && (
        <>
          <h3>Blocked</h3>
          <ul className="friend-list">{list.blocked.map((p) => person(p, [["unblock", "Unblock"]]))}</ul>
        </>
      )}
    </section>
  );
}

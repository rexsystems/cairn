import { useEffect, useState } from "react";
import { api, type DiscordStatus, type InviteInfo, type InviteTarget } from "../api";
import { useStore } from "../store";
import { Icon } from "./Icon";
import { Modal, Switch } from "./ui";

const HOW: Record<InviteTarget["kind"], string> = {
  playit: "Works for anyone on the internet.",
  lan: "Only for people on your Wi-Fi or network.",
  lanWorld: "Only for people on your Wi-Fi or network, while the world stays open to LAN.",
  server: "The server you're on now. Friends need to be allowed there too.",
};

/** Lets friends join this instance's session: from Discord when opted in, or with a copyable invite code. */
export function InviteDialog({ id, name, onClose }: { id: string; name: string; onClose: () => void }) {
  const { toast, settings } = useStore();
  const [targets, setTargets] = useState<InviteTarget[] | null>(null);
  const [address, setAddress] = useState("");
  const [discord, setDiscord] = useState<DiscordStatus | null>(null);
  const [onDiscord, setOnDiscord] = useState(false);
  const [info, setInfo] = useState<InviteInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.inviteTargets(id).then((t) => { setTargets(t); setAddress(t[0]?.address ?? ""); }).catch((e) => setError(String(e)));
    api.discordStatus().then((s) => { setDiscord(s); setOnDiscord(s.joinable.includes(id)); }).catch(() => {});
  }, [id]);

  const target = targets?.find((t) => t.address === address);
  useEffect(() => {
    setInfo(null);
    if (!target) return;
    let live = true;
    api.inviteEnable(id, target.address, target.max, onDiscord)
      .then((i) => live && setInfo(i))
      .catch((e) => { if (live) { setError(String(e)); setOnDiscord(false); } });
    return () => { live = false; };
  }, [id, target, onDiscord]);

  const toggleDiscord = async (on: boolean) => {
    setError(null);
    if (!on) await api.inviteDisable(id).catch(() => {});
    setOnDiscord(on);
  };

  const copy = async () => {
    if (!info) return;
    await navigator.clipboard.writeText(info.code);
    toast("Invite code copied");
  };

  const rpcOff = settings ? !settings.discordRpc : false;
  return (
    <Modal title={`Invite friends to ${name}`} onClose={onClose} footer={
      <>
        <button className="btn btn-quiet" onClick={onClose}>Close</button>
        <button className="btn" onClick={copy} disabled={!info} data-autofocus><Icon name="copy" />Copy invite code</button>
      </>
    }>
      {error && <div className="notice error" role="alert"><Icon name="warn" />{error}</div>}
      {!targets && !error && <p className="muted">Looking for an address friends can reach...</p>}
      {targets && targets.length === 0 && (
        <div className="notice" role="status">
          <Icon name="warn" />
          <p className="small">Nothing to invite to yet. Start the server in the Host tab (with network access or a playit.gg tunnel), join a server, or open a world to LAN from the pause menu.</p>
        </div>
      )}
      {targets && targets.length > 0 && (
        <>
          <fieldset className="stack invite-targets">
            <legend className="small muted">Friends connect to</legend>
            {targets.map((t) => (
              <label key={t.address} className="row">
                <input type="radio" name="invite-target" checked={t.address === address} onChange={() => setAddress(t.address)} />
                <span><b>{t.label}</b> <code className="mono">{t.address}</code><br /><span className="small muted">{HOW[t.kind]}</span></span>
              </label>
            ))}
          </fieldset>
          <Switch checked={onDiscord} onChange={toggleDiscord} disabled={rpcOff || !discord?.connected} label="Let Discord friends join from my profile" />
          {rpcOff && <p className="small muted">Turn on Discord status in Settings first.</p>}
          {!rpcOff && discord && !discord.connected && <p className="small muted">Open the Discord desktop app to use this.</p>}
          {onDiscord && (
            <p className="small">
              Friends click Join (or Ask to join) on your Discord profile or in a chat after you send an invite with the + button. Cairn opens on their side, sets up the same mods and starts the game into the server.
              The address stays in the hidden join secret; your status only shows the instance name and player count. This stays on until you close the game or stop the server.
            </p>
          )}
          <p className="small muted">Friend not on Discord? Send the invite code instead. They paste it in Library, Paste share code, and pick Create and join.</p>
          {info?.notes.map((n) => <p key={n} className="small muted"><Icon name="warn" size={14} /> {n}</p>)}
        </>
      )}
    </Modal>
  );
}

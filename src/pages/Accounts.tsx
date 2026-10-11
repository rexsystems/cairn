import { useEffect, useState } from "react";
import { api, type Account, type AccountCosmetics, type CapeOption } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { Avatar, Loading, Modal } from "../components/ui";
import { SkinPreview } from "../components/SkinPreview";
import { relativeTime } from "../format";

export function Accounts() {
  const { accounts, setAccounts, toast, settings } = useStore();
  const [starting, setStarting] = useState(false);
  const [signInError, setSignInError] = useState<string | null>(null);
  const [offlineName, setOfflineName] = useState("");
  const [offlineError, setOfflineError] = useState<string | null>(null);
  const [removing, setRemoving] = useState<Account | null>(null);
  const [manage, setManage] = useState<Account | null>(null);

  const ms = accounts.filter((a) => a.kind === "microsoft");
  const offline = accounts.filter((a) => a.kind === "offline");
  const reducedMotion = !!settings?.reduceMotion
    || (typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches);

  const begin = async () => {
    setStarting(true);
    setSignInError(null);
    try {
      const next = await api.msaLogin();
      if (next) {
        setAccounts(next);
        const active = next.find((a) => a.active);
        toast(active ? `Signed in as ${active.username}` : "Signed in");
      }
    } catch (e) {
      setSignInError(String(e));
    } finally {
      setStarting(false);
    }
  };

  const cancel = () => {
    api.msaCancel();
    setStarting(false);
  };

  const addOffline = async () => {
    setOfflineError(null);
    const n = offlineName.trim();
    if (!/^[A-Za-z0-9_]{3,16}$/.test(n)) { setOfflineError("Use 3 to 16 letters, numbers or underscores."); return; }
    try { setAccounts(await api.addOfflineAccount(n)); setOfflineName(""); toast(`Added offline account ${n}`); } catch (e) { setOfflineError(String(e)); }
  };

  const confirmRemove = async () => {
    if (!removing) return;
    try { setAccounts(await api.removeAccount(removing.id)); toast(`Removed ${removing.username}`); } catch (e) { toast(String(e), true); }
    setRemoving(null);
  };

  const row = (a: Account) => (
    <div key={a.id} className={`card account ${a.active ? "active" : ""}`}>
      <Avatar account={a} />
      <div style={{ minWidth: 0 }}>
        <div className="row" style={{ gap: 8 }}>
          <b style={{ fontSize: 16 }}>{a.username}</b>
          {a.active && <span className="chip chip-moss">Playing as</span>}
          {a.needsSignIn && <span className="chip chip-ember">Sign in again</span>}
        </div>
        <div className="small muted mono" style={{ overflow: "hidden", textOverflow: "ellipsis" }}>{a.uuid}</div>
        <div className="small muted">Added {relativeTime(a.added)}</div>
      </div>
      <div className="row" style={{ flexWrap: "nowrap" }}>
        {a.kind === "microsoft" && (
          <button className="btn btn-sm" onClick={() => setManage(a)}><Icon name="user" size={14} />Manage</button>
        )}
        {!a.active && <button className="btn btn-sm" onClick={async () => { try { setAccounts(await api.setActiveAccount(a.id)); toast(`Playing as ${a.username}`); } catch (e) { toast(String(e), true); } }}>Use this account</button>}
        <button className="btn btn-quiet btn-icon btn-sm" aria-label={`Remove ${a.username}`} title="Remove" onClick={() => setRemoving(a)}><Icon name="trash" size={16} /></button>
      </div>
    </div>
  );

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Accounts</h1>
          <p className="muted">Add as many accounts as you like and switch from the sidebar.</p>
        </div>
      </div>

      <section className="stack" aria-labelledby="off-h">
        <h2 id="off-h">Offline accounts</h2>
        <div className="panel stack">
          <p className="small muted">Play singleplayer, LAN, or servers with online mode off. No Microsoft sign-in required. Skins are not loaded for offline names.</p>
          <div className="row" style={{ alignItems: "flex-start" }}>
            <label className="field" style={{ flex: "1 1 240px" }}>
              <span>Player name</span>
              <input className="input" value={offlineName} maxLength={16} placeholder="Steve" aria-invalid={!!offlineError}
                onChange={(e) => setOfflineName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && addOffline()} />
              {offlineError && <span className="field-error">{offlineError}</span>}
            </label>
            <button className="btn btn-play" style={{ marginTop: 26 }} onClick={addOffline}><Icon name="plus" />Add offline account</button>
          </div>
        </div>
        {offline.map(row)}
      </section>

      <section className="stack" aria-labelledby="ms-h">
        <div className="row">
          <h2 id="ms-h" style={{ flex: 1 }}>Microsoft accounts</h2>
          <button className="btn" style={{ minHeight: 40, fontSize: 15 }} onClick={begin} disabled={starting}>
            <Icon name="plus" />{starting ? "Waiting for Microsoft..." : "Sign in with Microsoft"}
          </button>
        </div>
        {signInError && (
          <div className="notice error" role="alert"><Icon name="warn" /><div><b>Sign-in did not finish</b><p className="small">{signInError}</p></div></div>
        )}
        {starting && (
          <div className="notice" role="status">
            <Icon name="user" />
            <div>
              <b>Sign-in window is open</b>
              <p className="small muted">Finish signing in with Microsoft in that window. You can cancel here if you change your mind.</p>
              <button className="btn btn-quiet btn-sm" style={{ marginTop: 8 }} onClick={cancel}>Cancel sign-in</button>
            </div>
          </div>
        )}
        {ms.length === 0 && !starting && (
          <div className="panel"><p className="muted">Optional. Needed for online play and Realms. Open Manage on an account to preview skin and cape.</p></div>
        )}
        {ms.map(row)}
      </section>

      {manage && (
        <ManageAccount
          account={manage}
          reducedMotion={reducedMotion}
          onClose={() => setManage(null)}
          onAccounts={(list) => setAccounts(list)}
          onToast={toast}
        />
      )}

      {removing && (
        <Modal title={`Remove ${removing.username}?`} onClose={() => setRemoving(null)} footer={
          <>
            <button className="btn btn-quiet" onClick={() => setRemoving(null)}>Keep account</button>
            <button className="btn btn-danger" onClick={confirmRemove}><Icon name="trash" />Remove</button>
          </>
        }>
          <p>{removing.kind === "microsoft" ? "Cairn forgets the sign-in tokens for this account. Your Microsoft account itself is not affected." : "This offline name is removed. Worlds are kept."}</p>
        </Modal>
      )}
    </>
  );
}

function ManageAccount({
  account,
  onClose,
  onAccounts,
  onToast,
  reducedMotion,
}: {
  account: Account;
  onClose: () => void;
  onAccounts: (a: Account[]) => void;
  onToast: (t: string, err?: boolean) => void;
  reducedMotion: boolean;
}) {
  const [data, setData] = useState<AccountCosmetics | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [bump, setBump] = useState(0);

  const load = async () => {
    setError(null);
    try {
      const c = await api.accountCosmetics(account.id);
      setData(c);
      onAccounts(await api.listAccounts());
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => { void load(); }, [account.id]);

  const equip = async (capeId: string | null) => {
    setBusy(capeId ?? "none");
    try {
      const c = await api.setAccountCape(account.id, capeId);
      setData(c);
      setBump((n) => n + 1);
      onAccounts(await api.listAccounts());
      onToast(capeId ? "Cape equipped" : "Cape removed");
    } catch (e) {
      onToast(String(e), true);
    } finally {
      setBusy(null);
    }
  };

  const acc = data?.account ?? account;
  const capes = data?.capes ?? [];

  return (
    <Modal
      wide
      title={`Manage ${account.username}`}
      onClose={onClose}
      footer={<button className="btn" onClick={onClose}>Done</button>}
    >
      <div className="manage-account">
        <div className="manage-preview panel">
          {!data && !error && <Loading label="Loading skin" />}
          {error && <p className="field-error">{error}</p>}
          {data && (
            <SkinPreview
              skinUrl={acc.skinUrl}
              capeUrl={acc.capeUrl}
              model={acc.skinModel ?? "classic"}
              name={acc.username}
              bump={bump}
              reducedMotion={reducedMotion}
            />
          )}
        </div>
        <div className="stack" style={{ gap: 16 }}>
          <div>
            <h3 style={{ fontSize: 17, marginBottom: 6 }}>Cape</h3>
            <p className="small muted">Pick a cape you own. Changing it gives the preview a little spin.</p>
          </div>
          {!data && !error && <Loading label="Loading capes" />}
          {data && capes.length === 0 && (
            <div className="panel"><p className="muted">No capes on this account yet.</p></div>
          )}
          {data && (
            <div className="cape-grid">
              <button
                type="button"
                className={`cape-card${!acc.capeUrl ? " is-active" : ""}`}
                disabled={busy !== null}
                onClick={() => equip(null)}
                title="No cape"
              >
                <span className="cape-face cape-none" aria-hidden="true" />
                <span>No cape</span>
                {busy === "none" && <span className="small muted">Saving...</span>}
              </button>
              {capes.map((c: CapeOption) => (
                <button
                  key={c.id}
                  type="button"
                  className={`cape-card${c.active ? " is-active" : ""}`}
                  disabled={busy !== null}
                  onClick={() => equip(c.id)}
                  title={c.alias}
                >
                  <span className="cape-face" aria-hidden="true">
                    <img src={c.url} alt="" />
                  </span>
                  <span>{c.alias}</span>
                  {busy === c.id && <span className="small muted">Saving...</span>}
                </button>
              ))}
            </div>
          )}
          <button className="btn btn-quiet btn-sm" style={{ alignSelf: "flex-start" }} onClick={load} disabled={!!busy}>
            <Icon name="update" size={14} />Refresh from Minecraft
          </button>
        </div>
      </div>
    </Modal>
  );
}

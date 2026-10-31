import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, type Account, type AccountCosmetics, type CapeOption, type SkinOption } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { CapeThumb } from "../components/CapeThumb";
import { SkinThumb } from "../components/SkinThumb";
import { Avatar, Loading, Modal } from "../components/ui";
import { SkinPreview } from "../components/SkinPreview";
import { relativeTime } from "../format";

type AddStep = "pick" | "offline" | "microsoft";

export function Accounts() {
  const { accounts, setAccounts, toast, settings } = useStore();
  const [adding, setAdding] = useState<AddStep | null>(null);
  const [starting, setStarting] = useState(false);
  const [signInError, setSignInError] = useState<string | null>(null);
  const [offlineName, setOfflineName] = useState("");
  const [offlineError, setOfflineError] = useState<string | null>(null);
  const [removing, setRemoving] = useState<Account | null>(null);
  const [focusId, setFocusId] = useState<string | null>(null);

  const ms = accounts.filter((a) => a.kind === "microsoft");
  const active = accounts.find((a) => a.active) ?? null;
  const focused =
    (focusId ? accounts.find((a) => a.id === focusId) : null)
    ?? active
    ?? ms[0]
    ?? null;
  const reducedMotion = !!settings?.reduceMotion
    || (typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches);

  useEffect(() => {
    if (focusId && !accounts.some((a) => a.id === focusId)) setFocusId(null);
  }, [accounts, focusId]);

  const openAdd = () => {
    setSignInError(null);
    setOfflineError(null);
    setOfflineName("");
    setAdding("pick");
  };

  const closeAdd = () => {
    if (starting) {
      api.msaCancel();
      setStarting(false);
    }
    setAdding(null);
    setOfflineError(null);
    setOfflineName("");
  };

  const beginMicrosoft = async () => {
    setAdding("microsoft");
    setStarting(true);
    setSignInError(null);
    try {
      const next = await api.msaLogin();
      if (next) {
        setAccounts(next);
        const signed = next.find((a) => a.active);
        toast(signed ? `Signed in as ${signed.username}` : "Signed in");
        if (signed) setFocusId(signed.id);
        setAdding(null);
      } else {
        setAdding("pick");
      }
    } catch (e) {
      setSignInError(String(e));
      setAdding("pick");
    } finally {
      setStarting(false);
    }
  };

  const addOffline = async () => {
    setOfflineError(null);
    const n = offlineName.trim();
    if (!/^[A-Za-z0-9_]{3,16}$/.test(n)) {
      setOfflineError("Use 3 to 16 letters, numbers or underscores.");
      return;
    }
    try {
      const next = await api.addOfflineAccount(n);
      setAccounts(next);
      const added = next.find((a) => a.username === n && a.kind === "offline") ?? next.find((a) => a.username === n);
      if (added) setFocusId(added.id);
      toast(`Added offline account ${n}`);
      setAdding(null);
      setOfflineName("");
    } catch (e) {
      setOfflineError(String(e));
    }
  };

  const confirmRemove = async () => {
    if (!removing) return;
    try {
      setAccounts(await api.removeAccount(removing.id));
      toast(`Removed ${removing.username}`);
      if (focusId === removing.id) setFocusId(null);
    } catch (e) {
      toast(String(e), true);
    }
    setRemoving(null);
  };

  const useAccount = async (a: Account) => {
    try {
      setAccounts(await api.setActiveAccount(a.id));
      setFocusId(a.id);
      toast(`Playing as ${a.username}`);
    } catch (e) {
      toast(String(e), true);
    }
  };

  return (
    <>
      <div className="page-head">
        <div>
          <h1>Accounts</h1>
          <p className="muted">Skin and cape for the account you play as, then a short list to switch.</p>
        </div>
      </div>

      {signInError && !adding && (
        <div className="notice error" role="alert"><Icon name="warn" /><div><b>Sign-in did not finish</b><p className="small">{signInError}</p></div></div>
      )}
      {starting && !adding && (
        <div className="notice" role="status">
          <Icon name="user" />
          <div>
            <b>Sign-in window is open</b>
            <p className="small muted">Finish signing in with Microsoft in that window, or cancel below.</p>
            <button className="btn btn-quiet btn-sm" style={{ marginTop: 8 }} onClick={() => { api.msaCancel(); setStarting(false); }}>Cancel sign-in</button>
          </div>
        </div>
      )}

      {focused?.kind === "microsoft" ? (
        <AccountHero
          account={focused}
          reducedMotion={reducedMotion}
          onAccounts={setAccounts}
          onToast={toast}
          onRemove={() => setRemoving(focused)}
        />
      ) : focused?.kind === "offline" ? (
        <OfflineHero account={focused} onRemove={() => setRemoving(focused)} />
      ) : (
        !starting && (
          <div className="panel account-empty stack" style={{ gap: 14 }}>
            <p className="muted">No account yet. Sign in with the Microsoft account that owns Minecraft. After that you can add offline names for LAN and singleplayer.</p>
            <button type="button" className="btn btn-play" style={{ alignSelf: "flex-start" }} onClick={openAdd}>
              <Icon name="plus" />Add account
            </button>
          </div>
        )
      )}

      <section className="stack accounts-rest" aria-labelledby="rest-h">
        <div className="row" style={{ alignItems: "center" }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <h2 id="rest-h">All accounts</h2>
            <p className="small muted">Click a row to open it above.</p>
          </div>
          <button className="btn btn-play" style={{ minHeight: 40, fontSize: 15 }} onClick={openAdd} disabled={starting}>
            <Icon name="plus" />{starting ? "Waiting for Microsoft…" : "Add account"}
          </button>
        </div>

        <div className="account-list">
          {accounts.length === 0 && (
            <p className="muted" style={{ padding: "8px 2px" }}>Nothing saved yet.</p>
          )}
          {accounts.map((a) => (
            <div
              key={a.id}
              className={`account-row${focused?.id === a.id ? " is-open" : ""}${a.active ? " is-active" : ""}`}
            >
              <button type="button" className="account-row-open" onClick={() => setFocusId(a.id)}>
                <Avatar account={a} />
                <span className="account-row-main">
                  <span className="account-row-name">
                    {a.username}
                    {a.active && <span className="chip chip-moss">Playing as</span>}
                    {a.needsSignIn && <span className="chip chip-ember">Sign in again</span>}
                  </span>
                  <span className="small muted">{a.kind === "microsoft" ? "Microsoft" : "Offline"} · added {relativeTime(a.added)}</span>
                </span>
              </button>
              <span className="account-row-actions">
                {!a.active && (
                  <button type="button" className="btn btn-sm" onClick={() => useAccount(a)}>Use</button>
                )}
                <button
                  type="button"
                  className="btn btn-quiet btn-icon btn-sm"
                  aria-label={`Remove ${a.username}`}
                  title="Remove"
                  onClick={() => setRemoving(a)}
                >
                  <Icon name="trash" size={16} />
                </button>
              </span>
            </div>
          ))}
        </div>
      </section>

      {adding && (
        <Modal
          title={adding === "offline" ? "Offline account" : adding === "microsoft" ? "Microsoft sign-in" : "Add account"}
          onClose={starting ? () => { api.msaCancel(); setStarting(false); setAdding(null); } : closeAdd}
          footer={
            adding === "offline" ? (
              <>
                <button type="button" className="btn btn-quiet" onClick={() => { setAdding("pick"); setOfflineError(null); }}>Back</button>
                <button type="button" className="btn btn-play" onClick={() => void addOffline()}><Icon name="plus" />Add offline</button>
              </>
            ) : adding === "microsoft" ? (
              <button type="button" className="btn btn-quiet" onClick={() => { api.msaCancel(); setStarting(false); setAdding("pick"); }}>Cancel sign-in</button>
            ) : (
              <button type="button" className="btn btn-quiet" onClick={closeAdd}>Close</button>
            )
          }
        >
          {adding === "pick" && (
            <div className="stack" style={{ gap: 14 }}>
              <p className="muted">How do you want to play?</p>
              {signInError && <p className="field-error" role="alert">{signInError}</p>}
              <div className="add-account-choices">
                <button type="button" className="add-account-choice" onClick={() => void beginMicrosoft()} data-autofocus>
                  <span className="add-account-choice-icon" aria-hidden="true"><Icon name="globe" size={22} /></span>
                  <span className="add-account-choice-copy">
                    <strong>Microsoft</strong>
                    <span className="small muted">Online servers, Realms, skin and cape from your Minecraft profile.</span>
                  </span>
                </button>
                <button type="button" className="add-account-choice" onClick={() => setAdding("offline")} disabled={ms.length === 0}>
                  <span className="add-account-choice-icon" aria-hidden="true"><Icon name="user" size={22} /></span>
                  <span className="add-account-choice-copy">
                    <strong>Offline</strong>
                    <span className="small muted">
                      {ms.length === 0
                        ? "Sign in with a Microsoft account that owns Minecraft first."
                        : "LAN, singleplayer, or servers with online mode off."}
                    </span>
                  </span>
                </button>
              </div>
            </div>
          )}
          {adding === "offline" && (
            <div className="stack" style={{ gap: 12 }}>
              <p className="muted">Pick a player name. Skins are not loaded for offline names.</p>
              <label className="field">
                <span>Player name</span>
                <input
                  className="input"
                  value={offlineName}
                  maxLength={16}
                  placeholder="Steve"
                  data-autofocus
                  aria-invalid={!!offlineError}
                  onChange={(e) => setOfflineName(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && void addOffline()}
                />
                {offlineError && <span className="field-error">{offlineError}</span>}
              </label>
            </div>
          )}
          {adding === "microsoft" && (
            <div className="stack" style={{ gap: 10 }}>
              <p>A Microsoft sign-in window should be open. Finish there, then Cairn will add the account.</p>
              <p className="small muted">If nothing opened, cancel and try again.</p>
            </div>
          )}
        </Modal>
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

function OfflineHero({ account, onRemove }: { account: Account; onRemove: () => void }) {
  return (
    <section className="account-hero account-hero-offline panel" aria-labelledby="hero-name">
      <div className="account-hero-copy stack">
        <div className="row" style={{ gap: 10 }}>
          <Avatar account={account} />
          <div>
            <h2 id="hero-name" style={{ fontSize: 28 }}>{account.username}</h2>
            <p className="small muted">Offline · no skin or cape from Mojang</p>
          </div>
          {account.active && <span className="chip chip-moss">Playing as</span>}
        </div>
        <p className="muted">Offline names are for LAN and singleplayer. Sign in with Microsoft if you need online servers or Realms.</p>
        <div className="row">
          <button type="button" className="btn btn-quiet btn-sm" onClick={onRemove}>
            <Icon name="trash" size={14} />Remove
          </button>
        </div>
      </div>
    </section>
  );
}

function AccountHero({
  account,
  onAccounts,
  onToast,
  onRemove,
  reducedMotion,
}: {
  account: Account;
  onAccounts: (a: Account[]) => void;
  onToast: (t: string, err?: boolean) => void;
  onRemove: () => void;
  reducedMotion: boolean;
}) {
  const { setAccounts, toast } = useStore();
  const [data, setData] = useState<AccountCosmetics | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [bump, setBump] = useState(0);
  const [uploadModel, setUploadModel] = useState<"classic" | "slim">(
    account.skinModel === "slim" ? "slim" : "classic",
  );

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

  const acc = data?.account ?? account;
  const capes = data?.capes ?? [];
  const skins = data?.skins ?? [];

  useEffect(() => {
    setData(null);
    setBusy(null);
    setError(null);
    setUploadModel(account.skinModel === "slim" ? "slim" : "classic");
    void load();
  }, [account.id]);

  useEffect(() => {
    if (data?.account.skinModel) {
      setUploadModel(data.account.skinModel === "slim" ? "slim" : "classic");
    }
  }, [data?.account.skinModel]);

  const equip = async (capeId: string | null) => {
    if (busy) return;
    const current = data?.account ?? account;
    const list = data?.capes ?? [];
    const alreadyOff = capeId === null && !current.capeUrl;
    const alreadyOn = capeId !== null && list.some((c) => c.id === capeId && c.active);
    if (alreadyOff || alreadyOn) return;
    const token = capeId ?? "none";
    setBusy(token);
    try {
      const c = await api.setAccountCape(account.id, capeId);
      setData(c);
      setBump((n) => n + 1);
      onAccounts(await api.listAccounts());
      onToast(capeId ? "Cape equipped" : "Cape removed");
    } catch (e) {
      onToast(String(e), true);
    } finally {
      setBusy((b) => (b === token ? null : b));
    }
  };

  const makeActive = async () => {
    if (account.active) return;
    try {
      setAccounts(await api.setActiveAccount(account.id));
      toast(`Playing as ${account.username}`);
    } catch (e) {
      onToast(String(e), true);
    }
  };

  const changeSkin = async (model: "classic" | "slim") => {
    if (busy) return;
    const file = await open({
      multiple: false,
      filters: [{ name: "Minecraft skin", extensions: ["png"] }],
    });
    if (!file) return;
    const path = Array.isArray(file) ? file[0] : file;
    if (!path) return;
    setBusy("skin");
    try {
      const c = await api.setAccountSkin(account.id, path, model);
      setData(c);
      setBump((n) => n + 1);
      onAccounts(await api.listAccounts());
      onToast(`Skin updated (${model})`);
    } catch (e) {
      onToast(String(e), true);
    } finally {
      setBusy((b) => (b === "skin" ? null : b));
    }
  };

  const equipPastSkin = async (skin: SkinOption) => {
    if (busy || skin.active) return;
    const token = `skin:${skin.id}`;
    setBusy(token);
    try {
      const c = await api.setAccountSkinFromHistory(account.id, skin.id);
      setData(c);
      setBump((n) => n + 1);
      onAccounts(await api.listAccounts());
      onToast("Past skin equipped");
    } catch (e) {
      onToast(String(e), true);
    } finally {
      setBusy((b) => (b === token ? null : b));
    }
  };

  const model = (acc.skinModel === "slim" ? "slim" : "classic") as "classic" | "slim";

  return (
    <section className="account-hero panel" aria-labelledby="hero-name">
      <div className="account-hero-preview">
        {!data && !error && <Loading label="Loading skin" />}
        {error && <p className="field-error" style={{ padding: 16 }}>{error}</p>}
        {data && (
          <SkinPreview
            skinUrl={acc.skinUrl}
            capeUrl={acc.capeUrl}
            model={model}
            name={acc.username}
            bump={bump}
            reducedMotion={reducedMotion}
            size="lg"
          />
        )}
      </div>

      <div className="account-hero-side stack">
        <div>
          <div className="row" style={{ gap: 10, marginBottom: 6 }}>
            <h2 id="hero-name" style={{ fontSize: 28 }}>{acc.username}</h2>
            {account.active ? (
              <span className="chip chip-moss">Playing as</span>
            ) : (
              <button type="button" className="btn btn-sm btn-play" onClick={makeActive}>Use this account</button>
            )}
            {acc.needsSignIn && <span className="chip chip-ember">Sign in again</span>}
          </div>
          <p className="small muted mono" style={{ wordBreak: "break-all" }}>{acc.uuid}</p>
          <p className="small muted">Microsoft · added {relativeTime(acc.added)}</p>
        </div>

        <div className="stack" style={{ gap: 12 }}>
          <div>
            <h3 style={{ fontSize: 17, marginBottom: 4 }}>Skin</h3>
            <p className="small muted">Upload a 64×64 PNG. Classic is wide arms; slim is Alex-style.</p>
          </div>
          <div className="row" style={{ gap: 8, alignItems: "center" }}>
            <div className="segmented" role="group" aria-label="Arm model for upload">
              <button type="button" aria-pressed={uploadModel === "classic"} disabled={!!busy} onClick={() => setUploadModel("classic")}>
                Classic
              </button>
              <button type="button" aria-pressed={uploadModel === "slim"} disabled={!!busy} onClick={() => setUploadModel("slim")}>
                Slim
              </button>
            </div>
            <button
              type="button"
              className="btn btn-sm"
              disabled={!!busy}
              onClick={() => void changeSkin(uploadModel)}
            >
              <Icon name="image" size={14} />{busy === "skin" ? "Uploading…" : "Change skin"}
            </button>
          </div>
          {data && skins.length > 0 && (
            <div className="stack" style={{ gap: 8 }}>
              <p className="small muted">Past skins Cairn has seen on this account. Click one to re-equip.</p>
              <div className="skin-grid">
                {skins.map((s) => {
                  const token = `skin:${s.id}`;
                  return (
                    <button
                      key={s.id}
                      type="button"
                      className={`skin-card${s.active ? " is-active" : ""}${busy === token ? " is-busy" : ""}`}
                      disabled={busy !== null && busy !== token}
                      aria-busy={busy === token}
                      onClick={() => void equipPastSkin(s)}
                      title={s.active ? "Current skin" : `Equip ${s.model} skin`}
                    >
                      <SkinThumb url={s.url} />
                      <span className="small">{s.active ? "Current" : s.model}</span>
                      {busy === token && <span className="small muted">Saving…</span>}
                    </button>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        <div className="stack" style={{ gap: 12 }}>
          <div>
            <h3 style={{ fontSize: 17, marginBottom: 4 }}>Cape</h3>
            <p className="small muted">Front of each cape you own. Equipping one spins the preview.</p>
          </div>
          {!data && !error && <Loading label="Loading capes" />}
          {data && capes.length === 0 && (
            <p className="muted">No capes on this account yet.</p>
          )}
          {data && (
            <div className="cape-grid">
              <button
                type="button"
                className={`cape-card${!acc.capeUrl ? " is-active" : ""}${busy === "none" ? " is-busy" : ""}`}
                disabled={busy !== null && busy !== "none"}
                aria-busy={busy === "none"}
                onClick={() => void equip(null)}
                title="No cape"
              >
                <span className="cape-face cape-none" aria-hidden="true" />
                <span>No cape</span>
                {busy === "none" && <span className="small muted">Saving…</span>}
              </button>
              {capes.map((c: CapeOption) => (
                <button
                  key={c.id}
                  type="button"
                  className={`cape-card${c.active ? " is-active" : ""}${busy === c.id ? " is-busy" : ""}`}
                  disabled={busy !== null && busy !== c.id}
                  aria-busy={busy === c.id}
                  onClick={() => void equip(c.id)}
                  title={c.alias}
                >
                  <CapeThumb url={c.url} />
                  <span>{c.alias}</span>
                  {busy === c.id && <span className="small muted">Saving…</span>}
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="account-hero-actions row">
          <button className="btn btn-quiet btn-sm" onClick={load} disabled={!!busy}>
            <Icon name="update" size={14} />Refresh from Minecraft
          </button>
          <button type="button" className="btn btn-quiet btn-sm" onClick={onRemove}>
            <Icon name="trash" size={14} />Remove
          </button>
        </div>
      </div>
    </section>
  );
}

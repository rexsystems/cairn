import { useEffect, useState } from "react";
import { api, type Account } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { Avatar, Modal } from "../components/ui";
import { relativeTime } from "../format";
import { OfflineHero, AccountHero } from "./accounts/AccountHero";

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

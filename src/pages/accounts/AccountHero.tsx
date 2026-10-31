import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, type Account, type AccountCosmetics, type CapeOption, type SkinOption } from "../../api";
import { useStore } from "../../store";
import { Icon } from "../../components/Icon";
import { CapeThumb } from "../../components/CapeThumb";
import { SkinThumb } from "../../components/SkinThumb";
import { Avatar, Loading } from "../../components/ui";
import { SkinPreview } from "../../components/SkinPreview";
import { relativeTime } from "../../format";

export function OfflineHero({ account, onRemove }: { account: Account; onRemove: () => void }) {
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

export function AccountHero({
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

import { useEffect, useState } from "react";
import { api, type InstanceView, type SharedCode } from "../../api";
import { useStore } from "../../store";
import { Icon } from "../../components/Icon";
import { Modal } from "../../components/ui";

/** Share code for friends on Cairn, with the .mrpack file as the option for other launchers. */
export function ShareDialog({ inst, onExport, onClose }: { inst: InstanceView; onExport: () => void; onClose: () => void }) {
  const { toast } = useStore();
  const [shared, setShared] = useState<SharedCode | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.shareCode(inst.id).then(setShared).catch((e) => setError(String(e)));
  }, [inst.id]);

  const copy = async () => {
    if (!shared) return;
    await navigator.clipboard.writeText(shared.code);
    toast("Share code copied");
  };

  return (
    <Modal title={`Share ${inst.name}`} onClose={onClose} footer={
      <>
        <button className="btn btn-quiet" onClick={onExport}><Icon name="download" />Save as .mrpack instead</button>
        <button className="btn" onClick={copy} disabled={!shared} data-autofocus><Icon name="copy" />Copy code</button>
      </>
    }>
      <p className="muted">Friends paste this in Library, Paste share code. Cairn downloads the same versions from Modrinth and CurseForge, so the code stays short and holds no files.</p>
      {error && <div className="notice error" role="alert"><Icon name="warn" />{error}</div>}
      {!shared && !error && <p className="muted">Building the code...</p>}
      {shared && (
        <>
          <textarea className="input mono share-code" readOnly value={shared.code} aria-label="Share code" onFocus={(e) => e.currentTarget.select()} />
          <p className="small muted">{shared.items === 1 ? "1 file" : `${shared.items} files`} listed. Worlds, configs and options are not shared.</p>
          {shared.leftOut.length > 0 && (
            <div className="notice" role="status">
              <Icon name="warn" />
              <div>
                <b>{shared.leftOut.length === 1 ? "1 file is" : `${shared.leftOut.length} files are`} not in the code</b>
                <p className="small muted">They were added by hand, so there is nowhere to download them from. Send them separately or use the .mrpack file, which packs them in.</p>
                <ul className="small share-left-out">{shared.leftOut.map((f) => <li key={f}>{f}</li>)}</ul>
              </div>
            </div>
          )}
        </>
      )}
    </Modal>
  );
}

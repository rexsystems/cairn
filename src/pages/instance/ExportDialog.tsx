import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api, type InstanceView } from "../../api";
import { useStore } from "../../store";
import { Icon } from "../../components/Icon";
import { Modal } from "../../components/ui";

export function ExportDialog({ inst, onClose }: { inst: InstanceView; onClose: () => void }) {
  const { toast } = useStore();
  const [config, setConfig] = useState(true);
  const [options, setOptions] = useState(false);
  const [busy, setBusy] = useState(false);
  const run = async () => {
    const dest = await save({ defaultPath: `${inst.name}.mrpack`, filters: [{ name: "Modrinth modpack", extensions: ["mrpack"] }] });
    if (!dest) return;
    setBusy(true);
    try {
      const extra = [...(config ? ["config"] : []), ...(options ? ["options.txt"] : [])];
      const p = await api.exportMrpack(inst.id, dest, extra);
      toast(`Saved ${p}`);
      onClose();
    } catch (e) { toast(String(e), true); } finally { setBusy(false); }
  };
  return (
    <Modal title="Export as .mrpack" onClose={onClose} footer={
      <>
        <button className="btn btn-quiet" onClick={onClose}>Cancel</button>
        <button className="btn" onClick={run} disabled={busy}><Icon name="download" />{busy ? "Exporting..." : "Choose where to save"}</button>
      </>
    }>
      <p className="muted">Files that came from Modrinth are linked by URL and hash, so the pack stays small. Everything else is packed as an override.</p>
      <label className="check"><input type="checkbox" checked={config} onChange={(e) => setConfig(e.target.checked)} />Include the config folder</label>
      <label className="check"><input type="checkbox" checked={options} onChange={(e) => setOptions(e.target.checked)} />Include options.txt (your keybinds and video settings)</label>
    </Modal>
  );
}

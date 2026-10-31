import { useState } from "react";
import { api } from "../api";
import { useStore } from "../store";
import { blocksLaunch } from "../lib/install";
import { CompatIssues } from "./CompatIssues";
import { Icon } from "./Icon";
import { Modal } from "./ui";

export function PreLaunchDialog() {
  const { preLaunch, setPreLaunch, launchNow, instances } = useStore();
  const [rechecking, setRechecking] = useState(false);
  if (!preLaunch) return null;
  const name = instances?.find((i) => i.id === preLaunch.id)?.name ?? "This instance";
  const close = () => setPreLaunch(null);
  const recheck = async () => {
    setRechecking(true);
    try {
      const report = await api.checkCompat(preLaunch.id);
      if (blocksLaunch(report)) setPreLaunch({ ...preLaunch, report });
      else { close(); await launchNow(preLaunch.id, preLaunch.opts); }
    } finally { setRechecking(false); }
  };
  return (
    <Modal wide title={`${name} will probably crash`} onClose={close} footer={
      <>
        <button className="btn" onClick={close}>Cancel</button>
        <button className="btn" onClick={() => { close(); void launchNow(preLaunch.id, preLaunch.opts); }}>Launch anyway</button>
        <button className="btn btn-play" disabled={rechecking} onClick={recheck}><Icon name="play" />{rechecking ? "Checking..." : "Check again and play"}</button>
      </>
    }>
      <p className="muted" style={{ marginBottom: 12 }}>Cairn read the {preLaunch.report.mods} mods in this instance and found problems Minecraft will refuse to load.</p>
      <CompatIssues instanceId={preLaunch.id} issues={preLaunch.report.issues} onFixed={recheck} />
    </Modal>
  );
}

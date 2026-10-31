import { useState } from "react";
import { api } from "../api";
import { useStore } from "../store";
import { Modal, Switch } from "./ui";

/** First-run opt-in. Closing the dialog counts as "no" for both. */
export function PrivacyPrompt() {
  const { settings, setSettings, toast } = useStore();
  const [usage, setUsage] = useState(false);
  const [crashes, setCrashes] = useState(false);
  const [saving, setSaving] = useState(false);
  if (!settings || settings.privacyPromptAnswered) return null;

  const answer = async (telemetryEnabled: boolean, crashReportsEnabled: boolean) => {
    setSaving(true);
    try {
      setSettings(await api.saveSettings({ ...settings, telemetryEnabled, crashReportsEnabled, privacyPromptAnswered: true }));
    } catch (e) {
      toast(String(e), true);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      title="Help improve Cairn?"
      labelledBy="privacy-title"
      onClose={() => void answer(false, false)}
      footer={
        <>
          <button className="btn btn-quiet" disabled={saving} onClick={() => void answer(false, false)}>No thanks</button>
          <button className="btn" disabled={saving} data-autofocus onClick={() => void answer(usage, crashes)}>Save choice</button>
        </>
      }
    >
      <p>Cairn can send anonymous data to help fix bugs. Nothing is sent unless you turn it on, and you can change this later in Settings.</p>
      <Switch checked={usage} onChange={setUsage} label="Anonymous usage: app opens, game launches and installs, with version and OS" />
      <Switch checked={crashes} onChange={setCrashes} label="Crash and error reports" />
      <p className="small muted">Account names, worlds and files are never included.</p>
    </Modal>
  );
}

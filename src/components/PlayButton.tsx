import { useState } from "react";
import { api } from "../api";
import { useStore } from "../store";
import { Icon } from "./Icon";

export function PlayButton({ id, compact }: { id: string; compact?: boolean }) {
  const { games, play, accounts, go, toast } = useStore();
  const [busy, setBusy] = useState(false);
  const g = games[id];
  const state = g?.state ?? "exited";
  const noAccount = !accounts.some((a) => a.active);

  if (state === "running" || state === "preparing") {
    return (
      <button
        className={compact ? "btn btn-sm btn-danger" : "btn btn-stop"}
        aria-live="polite"
        onClick={async (e) => {
          e.stopPropagation();
          await api.kill(id);
          toast(state === "preparing" ? "Cancelling launch" : "Stopping the game");
        }}
      >
        <Icon name="stop" size={compact ? 14 : 18} /> {state === "preparing" ? "Cancel" : "Stop"}
      </button>
    );
  }
  if (noAccount) {
    return (
      <button className={compact ? "btn btn-sm" : "btn btn-play"} onClick={(e) => { e.stopPropagation(); go({ name: "accounts" }); }} title="Add an offline or Microsoft account before playing">
        <Icon name="user" size={compact ? 14 : 18} /> {compact ? "Account" : "Add account to play"}
      </button>
    );
  }
  return (
    <button
      className={compact ? "btn btn-sm btn-play" : "btn btn-play"}
      style={compact ? { minHeight: 32, padding: "0 12px", fontSize: 14 } : undefined}
      disabled={busy}
      onClick={async (e) => { e.stopPropagation(); setBusy(true); await play(id); setBusy(false); }}
    >
      <Icon name="play" size={compact ? 14 : 18} /> Play
    </button>
  );
}

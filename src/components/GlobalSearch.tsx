import { useEffect, useState } from "react";
import { api, type ContentHit } from "../api";
import { useStore } from "../store";
import { Modal, Loading, EmptyState } from "./ui";
import { Icon } from "./Icon";

const KIND_LABEL: Record<string, string> = {
  mods: "Mod",
  resourcepacks: "Resource pack",
  shaderpacks: "Shader",
  datapacks: "Datapack",
};

export function GlobalSearch({ onClose }: { onClose: () => void }) {
  const { go } = useStore();
  const [q, setQ] = useState("");
  const [debounced, setDebounced] = useState("");
  const [hits, setHits] = useState<ContentHit[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const t = setTimeout(() => setDebounced(q.trim()), 220);
    return () => clearTimeout(t);
  }, [q]);

  useEffect(() => {
    if (!debounced) { setHits([]); setError(null); return; }
    setHits(null);
    api.searchLibraryContent(debounced)
      .then(setHits)
      .catch((e) => setError(String(e)));
  }, [debounced]);

  return (
    <Modal title="Search library" onClose={onClose} wide>
      <label className="search">
        <span className="sr-only">Search installed content</span>
        <Icon name="search" size={16} />
        <input
          className="input"
          data-autofocus
          placeholder="Where is Sodium installed?"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
      </label>
      {error && <p className="field-error">{error}</p>}
      {hits === null && debounced && <Loading label="Searching instances" />}
      {hits && hits.length === 0 && debounced && (
        <EmptyState icon="search" title="Nothing matches">No installed mod, pack, or shader uses that name.</EmptyState>
      )}
      {hits && hits.length > 0 && (
        <div className="card import-list" style={{ maxHeight: "min(50dvh, 440px)" }}>
          {hits.map((h) => (
            <button
              key={`${h.instanceId}:${h.kind}:${h.fileName}`}
              type="button"
              className="import-row"
              onClick={() => { onClose(); go({ name: "instance", id: h.instanceId, tab: "content" }); }}
            >
              <span className="import-row-main">
                <span className="import-row-name">{h.title}</span>
                <span className="small muted">{KIND_LABEL[h.kind] ?? h.kind} · {h.instanceName} · {h.fileName}</span>
              </span>
            </button>
          ))}
        </div>
      )}
    </Modal>
  );
}

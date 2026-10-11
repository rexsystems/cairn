import { fileSrc, type InstanceView } from "../api";
import { useStore } from "../store";
import { InstanceIcon } from "./ui";
import { PlayButton } from "./PlayButton";
import { loaderLabel, playTime, relativeTime } from "../format";

export function InstanceCard({ inst: i }: { inst: InstanceView }) {
  const { go, games } = useStore();
  const g = games[i.id];
  const open = () => go({ name: "instance", id: i.id });
  return (
    <div className="card inst-card" role="button" tabIndex={0} aria-label={`Open ${i.name}`}
      onClick={open}
      onKeyDown={(e) => { if (e.target === e.currentTarget && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); open(); } }}>
      <div className={`inst-cover tone-${i.loader}`} aria-hidden="true">
        {i.cover && <img src={fileSrc(i.cover)} alt="" loading="lazy" />}
      </div>
      <div className="inst-card-body">
        <div className="inst-card-top">
          <InstanceIcon name={i.name} src={i.iconPath} />
          <div style={{ minWidth: 0 }}>
            <div className="inst-card-name">{i.name}</div>
            <div className="small muted">{loaderLabel[i.loader]} {i.mcVersion}</div>
          </div>
        </div>
        <div className="row small muted" style={{ gap: 12 }}>
          {g && g.state !== "exited" ? <span className="chip chip-moss">{g.state === "running" ? "Running" : "Starting"}</span> : <span>{i.lastPlayed ? `Played ${relativeTime(i.lastPlayed)}` : "Not played yet"}</span>}
          {i.modCount > 0 && <span>{i.modCount} {i.modCount === 1 ? "mod" : "mods"}</span>}
          {i.playSeconds >= 60 && <span>{playTime(i.playSeconds)}</span>}
        </div>
      </div>
      <div className="play-fab"><PlayButton id={i.id} compact /></div>
    </div>
  );
}

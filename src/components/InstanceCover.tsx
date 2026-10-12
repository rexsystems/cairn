import { useEffect, useState, type ReactNode } from "react";
import { fileSrc, type InstanceView } from "../api";
import { resolveCover } from "../coverArt";

/** Cover art: screenshot → Modrinth gallery → wiki version art → CSS tone. */
export function InstanceCover({
  inst,
  className,
  children,
}: {
  inst: InstanceView;
  className?: string;
  children?: ReactNode;
}) {
  const [src, setSrc] = useState<string | null>(() => (inst.cover && fileSrc(inst.cover)) || null);

  useEffect(() => {
    let alive = true;
    setSrc((inst.cover && fileSrc(inst.cover)) || null);
    resolveCover(inst).then((url) => {
      if (alive) setSrc(url);
    });
    return () => { alive = false; };
  }, [inst.id, inst.cover, inst.pack?.projectId, inst.mcVersion]);

  return (
    <div className={className ?? `inst-cover tone-${inst.loader}`}>
      {src && <img src={src} alt="" loading="lazy" />}
      {children}
    </div>
  );
}

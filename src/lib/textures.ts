import { useEffect, useState } from "react";
import { api } from "../api";

// Mojang texture links are plain http with no CORS, which the CSP and WebGL refuse, so the
// backend downloads and caches them and the UI only ever sees data URLs.
const cache = new Map<string, Promise<string>>();

export function textureSrc(url: string): Promise<string> {
  if (url.startsWith("data:") || url.startsWith("blob:")) return Promise.resolve(url);
  let hit = cache.get(url);
  if (!hit) {
    hit = api.texture(url);
    hit.catch(() => cache.delete(url));
    cache.set(url, hit);
  }
  return hit;
}

/** `undefined` while loading or when there is no texture; `failed` once the download gave up. */
export function useTexture(url?: string | null): { src?: string; failed: boolean } {
  const [state, setState] = useState<{ url?: string | null; src?: string; failed: boolean }>({ url, failed: false });
  useEffect(() => {
    if (!url) return;
    let live = true;
    textureSrc(url).then(
      (src) => live && setState({ url, src, failed: false }),
      () => live && setState({ url, failed: true }),
    );
    return () => { live = false; };
  }, [url]);
  return state.url === url ? { src: state.src, failed: state.failed } : { failed: false };
}

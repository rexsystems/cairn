// WebKitGTK sometimes refuses <img> loads from the asset: scheme when the page origin is different
// (the Vite dev server, for one) even though fetch() on the same URL succeeds. When a local image
// fails, retry once through fetch and a blob URL.
const retried = new WeakSet<HTMLImageElement>();

document.addEventListener("error", (e) => {
  const img = e.target as HTMLImageElement;
  if (img?.tagName !== "IMG" || retried.has(img)) return;
  const src = img.currentSrc || img.src;
  if (!/^(asset:|https?:\/\/asset\.localhost)/.test(src)) return;
  retried.add(img);
  fetch(src)
    .then((r) => (r.ok ? r.blob() : Promise.reject(r.status)))
    .then((b) => { img.src = URL.createObjectURL(b); })
    .catch(() => { img.style.visibility = "hidden"; });
}, true);

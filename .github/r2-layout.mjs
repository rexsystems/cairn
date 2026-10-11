/**
 * R2 object layout for Cairn updater artifacts.
 *
 *   latest.json                         ← always at bucket root
 *   builds/v0.1.0-build.12/<artifact>   ← one folder per stamped release
 */

/** `0.1.0+12` or `v0.1.0-build.12` → `builds/v0.1.0-build.12` */
export function buildPrefixFromVersion(version) {
  const raw = String(version || "").trim();
  if (!raw) return "builds/unknown";

  const tagged = raw.match(/^v?\d+\.\d+\.\d+-build\.\d+$/i);
  if (tagged) {
    const tag = raw.startsWith("v") || raw.startsWith("V") ? raw : `v${raw}`;
    return `builds/${tag}`;
  }

  const stamped = raw.match(/^(\d+\.\d+\.\d+)\+(\d+)$/);
  if (stamped) return `builds/v${stamped[1]}-build.${stamped[2]}`;

  const plain = raw.match(/^v?(\d+\.\d+\.\d+)$/);
  if (plain) return `builds/v${plain[1]}`;

  return `builds/${raw.replace(/[^a-zA-Z0-9._+-]+/g, "-")}`;
}

export function encodeArtifactPath(objectKey) {
  return String(objectKey)
    .split("/")
    .map((part) => part.replace(/\+/g, "%2B"))
    .join("/");
}

export function artifactPublicUrl(baseUrl, objectKey) {
  const base = String(baseUrl || "").replace(/\/$/, "");
  return `${base}/${encodeArtifactPath(objectKey)}`;
}

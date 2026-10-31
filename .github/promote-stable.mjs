#!/usr/bin/env node
/**
 * Picks the build to put on the stable channel and checks that every file it
 * points to is still on R2. Writes the manifest to <out-file>.
 *
 * Env:
 *   UPDATER_DOWNLOAD_BASE_URL  public R2 URL
 *   RELEASE_VERSION            X.Y.Z from the tag; the build's base version must match
 *   BUILD_TAG                  optional vX.Y.Z-build.N; defaults to the current beta
 */
import { writeFileSync } from "node:fs";
import { encodeArtifactPath } from "./r2-layout.mjs";

const out = process.argv[2];
const base = process.env.UPDATER_DOWNLOAD_BASE_URL;
const want = process.env.RELEASE_VERSION;
const buildTag = process.env.BUILD_TAG?.trim();
if (!out || !base || !/^\d+\.\d+\.\d+$/.test(want ?? "")) {
  console.error("Usage: RELEASE_VERSION=X.Y.Z UPDATER_DOWNLOAD_BASE_URL=... node .github/promote-stable.mjs <out-file>");
  process.exit(1);
}

const source = buildTag
  ? `${base}/${encodeArtifactPath(`builds/${buildTag}/latest.json`)}`
  : `${base}/beta/latest.json`;
const res = await fetch(source, { cache: "no-store" });
if (!res.ok) {
  console.error(`Could not read ${source}: HTTP ${res.status}`);
  process.exit(1);
}
const manifest = await res.json();
const got = String(manifest.version ?? "").split("+")[0];
if (got !== want) {
  console.error(`${source} is version ${manifest.version}, but the tag asks for ${want}.`);
  console.error("Push the version bump to main first and wait for its build, or pass build_tag.");
  process.exit(1);
}

const platforms = Object.entries(manifest.platforms ?? {});
if (platforms.length === 0) {
  console.error(`${source} lists no platforms`);
  process.exit(1);
}
for (const [name, p] of platforms) {
  const head = await fetch(p.url, { method: "HEAD" });
  if (!head.ok) {
    console.error(`${name}: ${p.url} returned HTTP ${head.status}`);
    process.exit(1);
  }
}

writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Promoting ${manifest.version} (${platforms.length} platforms) from ${source}`);

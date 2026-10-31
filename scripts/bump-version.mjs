#!/usr/bin/env node
/**
 * Sets the base version (X.Y.Z) everywhere and moves the CHANGELOG "Unreleased"
 * notes under a heading for that version. CI adds "+<build>" on its own.
 *
 *   npm run bump -- 0.2.0
 */
import { readFileSync, writeFileSync } from "node:fs";

const next = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(next ?? "")) {
  console.error("Usage: npm run bump -- <X.Y.Z>");
  process.exit(1);
}

function edit(file, change) {
  const before = readFileSync(file, "utf8");
  const after = change(before);
  if (after === before) {
    console.error(`No version found to change in ${file}`);
    process.exit(1);
  }
  writeFileSync(file, after);
}

const jsonVersion = (s) => s.replace(/^(\s*"version":\s*)"[^"]+"/m, `$1"${next}"`);
edit("package.json", jsonVersion);
edit("src-tauri/tauri.conf.json", jsonVersion);
edit("src-tauri/Cargo.toml", (s) => s.replace(/^version\s*=\s*"[^"]+"/m, `version = "${next}"`));
edit("src-tauri/Cargo.lock", (s) => s.replace(/(\nname = "cairn"\nversion = )"[^"]+"/, `$1"${next}"`));
edit("package-lock.json", (s) =>
  s
    .replace(/^(\s*"version":\s*)"[^"]+"/m, `$1"${next}"`)
    .replace(/("packages":\s*\{\s*"":\s*\{[^}]*?"version":\s*)"[^"]+"/, `$1"${next}"`),
);

const today = new Date().toISOString().slice(0, 10);
edit("CHANGELOG.md", (s) => {
  if (s.includes(`## [${next}]`)) {
    console.error(`CHANGELOG.md already has a section for ${next}`);
    process.exit(1);
  }
  return s.replace("## [Unreleased]\n", `## [Unreleased]\n\n## [${next}] - ${today}\n`);
});

console.log(`Version set to ${next}. Commit, push to main, then tag v${next} once a build looks good.`);

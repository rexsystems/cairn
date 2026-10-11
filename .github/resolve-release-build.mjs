#!/usr/bin/env node
/**
 * Resolve the next per-base build number for a release.
 * Tags: v0.1.0-build.N  →  stamped app version 0.1.0+N
 */
import { appendFileSync, readFileSync } from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";

const confPath = path.resolve("src-tauri/tauri.conf.json");
const conf = JSON.parse(readFileSync(confPath, "utf8"));
const base = String(conf.version ?? "0.0.0").split("+")[0].split("-")[0];
if (!/^\d+\.\d+\.\d+$/.test(base)) {
  console.error(`Invalid base version in tauri.conf.json: ${base}`);
  process.exit(1);
}

const tagPrefix = `v${base}-build.`;
let maxBuild = 0;

function considerTag(name) {
  if (!name.startsWith(tagPrefix)) return;
  const n = Number(name.slice(tagPrefix.length));
  if (Number.isInteger(n) && n > maxBuild) maxBuild = n;
}

function scanLocalTags() {
  try {
    const listed = execSync(`git tag -l "${tagPrefix}*"`, { encoding: "utf8" });
    for (const name of listed.split(/\r?\n/)) {
      if (name) considerTag(name);
    }
  } catch {
    /* first release */
  }
}

const repo = process.env.GITHUB_REPOSITORY;
if (repo) {
  try {
    const apiPath = `repos/${repo}/git/matching-refs/tags/${tagPrefix}`;
    const raw = execSync(`gh api --paginate "${apiPath}"`, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      env: process.env,
    });
    const refs = raw.trim() ? JSON.parse(raw) : [];
    for (const ref of Array.isArray(refs) ? refs : []) {
      const name = String(ref.ref ?? "").replace(/^refs\/tags\//, "");
      considerTag(name);
    }
  } catch (err) {
    console.error(
      "gh api tag lookup failed; falling back to local tags:",
      err?.stderr?.toString?.() || err.message,
    );
    scanLocalTags();
  }
} else {
  scanLocalTags();
}

const build = maxBuild + 1;
const stamped = `${base}+${build}`;
const tag = `v${base}-build.${build}`;

console.log(`Resolved ${stamped} (previous max for ${base}: ${maxBuild || "none"})`);

const out = process.env.GITHUB_OUTPUT;
if (out) {
  appendFileSync(out, `base=${base}\nbuild=${build}\nstamped=${stamped}\ntag=${tag}\n`);
} else {
  console.log(`base=${base}`);
  console.log(`build=${build}`);
  console.log(`stamped=${stamped}`);
  console.log(`tag=${tag}`);
}

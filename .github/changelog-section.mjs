#!/usr/bin/env node
/** Prints the CHANGELOG.md notes for one version: `node .github/changelog-section.mjs 0.2.0` */
import { readFileSync } from "node:fs";

const version = process.argv[2];
const lines = readFileSync("CHANGELOG.md", "utf8").split("\n");
const start = lines.findIndex((l) => l.startsWith(`## [${version}]`));
if (start < 0) {
  console.error(`CHANGELOG.md has no section for ${version}`);
  process.exit(1);
}
const end = lines.findIndex((l, i) => i > start && l.startsWith("## "));
const body = lines.slice(start + 1, end < 0 ? undefined : end).join("\n").trim();
console.log(body || "No notes.");

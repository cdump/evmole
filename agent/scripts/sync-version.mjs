#!/usr/bin/env node

import { readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const check = process.argv.includes("--check");
const packagePath = resolve(repositoryRoot, "javascript/package.json");
const canonical = await readJson(packagePath);
const version = canonical.version;
const changes = [];

await synchronizeJson("javascript/package-lock.json", (value) => {
  value.version = version;
  value.packages[""].version = version;
});
await synchronizeJson("agent/mcp/package.json", (value) => {
  value.version = version;
  value.dependencies.evmole = version;
});
await synchronizeJson("agent/mcp/package-lock.json", (value) => {
  value.version = version;
  value.packages[""].version = version;
  value.packages[""].dependencies.evmole = version;
  const evmoleEntry = value.packages["node_modules/evmole"];
  if (evmoleEntry && evmoleEntry.version !== version) {
    evmoleEntry.version = version;
    delete evmoleEntry.resolved;
    delete evmoleEntry.integrity;
  }
});
await synchronizeJson("agent/mcp/server.json", (value) => {
  value.version = version;
  for (const packageEntry of value.packages) packageEntry.version = version;
});

if (check && changes.length > 0) {
  process.stderr.write(`Version drift detected:\n${changes.map((path) => `- ${path}`).join("\n")}\n`);
  process.exitCode = 1;
} else if (check) {
  process.stdout.write(`All coordinated artifacts use version ${version}.\n`);
} else {
  process.stdout.write(`Synchronized coordinated artifacts to version ${version}.\n`);
}

async function synchronizeJson(path, update) {
  const absolute = resolve(repositoryRoot, path);
  const current = await readJson(absolute);
  const next = structuredClone(current);
  update(next);
  const serialized = `${JSON.stringify(next, null, 2)}\n`;
  const existing = await readFile(absolute, "utf8");
  if (serialized !== existing) {
    changes.push(path);
    if (!check) await writeFile(absolute, serialized, "utf8");
  }
}

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

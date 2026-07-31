#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFile, stat } from "node:fs/promises";

const json = async (relative) =>
  JSON.parse(await readFile(new URL(`../${relative}`, import.meta.url), "utf8"));
const exists = async (relative) =>
  (await stat(new URL(`../${relative}`, import.meta.url))).isFile();

const server = await json("mcp/server.json");
const mcpPackage = await json("mcp/package.json");

assert.equal(mcpPackage.name, "evmole-mcp");
assert.equal(mcpPackage.dependencies.evmole, mcpPackage.version);
assert.equal(mcpPackage.bin["evmole-mcp"], "./src/server.mjs");
assert.equal(server.name, mcpPackage.mcpName);
assert.equal(server.version, mcpPackage.version);
assert.equal(server.repository.subfolder, "agent/mcp");
assert.equal(server.packages.length, 1);
assert.equal(server.packages[0].identifier, mcpPackage.name);
assert.equal(server.packages[0].version, mcpPackage.version);
assert.equal(server.remotes, undefined);
assert.deepEqual(server.packages.map((entry) => entry.transport.type), ["stdio"]);
assert.ok(server.packages.every((entry) => entry.environmentVariables === undefined));
assert.equal(await exists("skills/evm-bytecode-analysis/SKILL.md"), true);
assert.equal(
  await exists("skills/evm-bytecode-analysis/references/libraries.md"),
  true,
);

for (const path of [
  "mcp/package.json",
  "mcp/server.json",
]) {
  const content = await readFile(new URL(`../${path}`, import.meta.url), "utf8");
  assert.doesNotMatch(content, /\/home\/|[A-Za-z]:\\\\/);
}

console.log("Agent skill, npm MCP package, and MCP Registry metadata are valid.");

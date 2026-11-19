#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const skillUrl = new URL("../skills/evm-bytecode-analysis/SKILL.md", import.meta.url);
const skill = await readFile(skillUrl, "utf8");
const agentGuideUrl = new URL("../../AGENTS.md", import.meta.url);
const agentGuide = await readFile(agentGuideUrl, "utf8");
const librariesUrl = new URL(
  "../skills/evm-bytecode-analysis/references/libraries.md",
  import.meta.url,
);
const libraries = await readFile(librariesUrl, "utf8");

for (const section of [
  "selectors",
  "arguments",
  "stateMutability",
  "storage",
  "transientStorage",
  "metadata",
  "proxy",
  "controlFlowGraph",
]) {
  assert.match(skill, new RegExp(section), `skill must route ${section}`);
}
assert.match(skill, /MCP/i);
assert.match(skill, /CLI/i);
assert.match(skill, /runtime bytecode/i);
assert.match(skill, /inferred/i);
assert.match(skill, /truncat/i);
assert.match(skill, /Route by intent/i);
assert.match(skill, /references\/libraries\.md/i);
assert.match(skill, /separate application/i);
assert.match(skill, /EVMole's own source code/i);
for (const language of ["Rust", "Go", "Python", "JavaScript"]) {
  assert.match(skill, new RegExp(language, "i"), `skill must route ${language} embedding`);
}
for (const [pattern, binding] of [
  [/\bcargo add evmole\b/, "Rust"],
  [/\bnpm install evmole\b/, "JavaScript"],
  [/\bpip install --upgrade evmole\b/, "Python"],
  [/\bgo get github\.com\/cdump\/evmole\/go\b/, "Go"],
]) {
  assert.match(libraries, pattern, `library reference must route the ${binding} binding`);
}
for (const [pattern, capability] of [
  [/calldata decoding/i, "reject calldata decoding"],
  [/address\/RPC fetching/i, "reject address and RPC fetching"],
  [/source verification/i, "reject source verification"],
  [/exact source reconstruction/i, "reject exact source reconstruction"],
  [/non-EVM input/i, "reject non-EVM input"],
  [/full\s+(?:source-code\s+)?decompilation/i, "reject full decompilation"],
]) {
  assert.match(skill, pattern, `skill must ${capability}`);
}

const contributorCases = [
  {
    prompt: "Add a new CFG resolution algorithm to EVMole",
    routes: [/Core analysis algorithms/i, /`src\/`/],
  },
  {
    prompt: "Expose an EVMole result through the Python binding",
    routes: [/Python bindings/i, /`src\/interface_py\.rs`/, /`python\/`/],
  },
  {
    prompt: "Change pagination in the EVMole MCP response",
    routes: [/shared agent adapter/i, /`javascript\/src\/agent_api\.mjs`/, /`agent\/`/],
  },
];

for (const { prompt, routes } of contributorCases) {
  for (const route of routes) {
    assert.match(agentGuide, route, `AGENTS.md must route contributor request: ${prompt}`);
  }
}

assert.match(agentGuide, /implementation repository/i);
assert.match(agentGuide, /only when the user asks to\s+analyze supplied deployed runtime bytecode/i);
assert.match(agentGuide, /When helping a separate project integrate EVMole/i);
assert.doesNotMatch(
  agentGuide,
  /^- Read `agent\/skills\/evm-bytecode-analysis\/SKILL\.md`/m,
  "AGENTS.md must not require the consumer skill for every repository task",
);

console.log("Validated consumer and contributor routing coverage.");

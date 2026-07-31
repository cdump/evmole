#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const skillUrl = new URL("../skills/evm-bytecode-analysis/SKILL.md", import.meta.url);
const skill = await readFile(skillUrl, "utf8");
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

console.log("Validated supported and unsupported skill routing coverage.");

#!/usr/bin/env node

import assert from "node:assert/strict";

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const version = process.argv[2];
assert.match(version ?? "", /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/);

const transport = new StdioClientTransport({
  command: "npx",
  args: ["-y", `evmole-mcp@${version}`],
  stderr: "pipe",
});
const client = new Client({
  name: "evmole-release-smoke",
  version: "1.0.0",
});

await client.connect(transport);
try {
  const tools = await client.listTools();
  assert.deepEqual(tools.tools.map((tool) => tool.name), [
    "analyze_evm_bytecode",
  ]);
  const result = await client.callTool({
    name: "analyze_evm_bytecode",
    arguments: { bytecode: "0x00", include: [] },
  });
  assert.equal(result.isError, undefined);
  assert.equal(result.structuredContent.evmoleVersion, version);
  assert.equal(result.structuredContent.input.byteLength, 1);
} finally {
  await client.close();
}

process.stdout.write(`Public evmole-mcp@${version} stdio smoke test passed.\n`);

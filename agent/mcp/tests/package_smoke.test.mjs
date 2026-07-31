import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const mcpRoot = resolve(new URL("..", import.meta.url).pathname);
const javascriptRoot = resolve(mcpRoot, "../../javascript");

function pack(root, environment) {
  const result = spawnSync("npm", ["pack", "--json"], {
    cwd: root,
    encoding: "utf8",
    env: environment,
  });
  assert.equal(result.status, 0, result.stderr || String(result.error));
  const parsed = JSON.parse(result.stdout);
  return Array.isArray(parsed) ? parsed[0] : Object.values(parsed)[0];
}

test("packed MCP executable works with packed evmole", { timeout: 120_000 }, async () => {
  const directory = await mkdtemp(join(tmpdir(), "evmole-mcp-package-"));
  const environment = {
    ...process.env,
    npm_config_cache: join(directory, "npm-cache"),
  };
  delete environment.NODE_TEST_CONTEXT;
  const evmole = pack(javascriptRoot, environment);
  const mcp = pack(mcpRoot, environment);
  const paths = new Set(mcp.files.map((file) => file.path));
  assert.deepEqual(
    [...paths].sort(),
    ["LICENSE", "README.md", "package.json", "server.json", "src/server.mjs"],
  );

  for (const tarball of [
    join(javascriptRoot, basename(evmole.filename)),
    join(mcpRoot, basename(mcp.filename)),
  ]) {
    const install = spawnSync(
      "npm",
      ["install", "--ignore-scripts", "--no-audit", "--no-fund", tarball],
      { cwd: directory, encoding: "utf8", env: environment },
    );
    assert.equal(install.status, 0, install.stderr || String(install.error));
  }

  const executable = process.platform === "win32"
    ? join(directory, "node_modules", ".bin", "evmole-mcp.cmd")
    : join(directory, "node_modules", ".bin", "evmole-mcp");
  const transport = new StdioClientTransport({
    command: executable,
    args: [],
    cwd: directory,
    env: environment,
    stderr: "pipe",
  });
  const client = new Client({ name: "packed-evmole-mcp-test", version: "1.0.0" });
  await client.connect(transport);
  try {
    assert.equal(client.getServerVersion().version, mcp.version);
    const result = await client.callTool({
      name: "analyze_evm_bytecode",
      arguments: { bytecode: "0x00", include: [] },
    });
    assert.equal(result.structuredContent.schemaVersion, 1);
    assert.equal(result.structuredContent.evmoleVersion, evmole.version);
    assert.deepEqual(JSON.parse(result.content[0].text), result.structuredContent);
  } finally {
    await client.close();
  }
  assert.equal(transport.stderr.read(), null);
});

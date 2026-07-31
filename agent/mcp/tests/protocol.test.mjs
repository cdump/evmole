import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import test from "node:test";

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const runner = fileURLToPath(new URL("./run-local-server.mjs", import.meta.url));
const cli = fileURLToPath(
  new URL("../../../javascript/dist/evmole_cli.mjs", import.meta.url),
);
const fixture = async (name) =>
  (await readFile(
    new URL(`../../tests/fixtures/${name}`, import.meta.url),
    "utf8",
  )).trim();
const childEnvironment = { ...process.env };
delete childEnvironment.NODE_TEST_CONTEXT;

async function withClient(callback, command = process.execPath, args = [runner]) {
  const transport = new StdioClientTransport({
    command,
    args,
    env: childEnvironment,
    stderr: "pipe",
  });
  const client = new Client({ name: "evmole-mcp-test", version: "1.0.0" });
  await client.connect(transport);
  try {
    return await callback(client, transport);
  } finally {
    await client.close();
  }
}

test("negotiates protocol and lists exactly one documented tool", async () => {
  await withClient(async (client, transport) => {
    const result = await client.listTools();
    assert.equal(result.tools.length, 1);
    assert.equal(result.tools[0].name, "analyze_evm_bytecode");
    assert.match(result.tools[0].description, /deployed EVM runtime bytecode/);
    assert.equal(result.tools[0].inputSchema.required.includes("bytecode"), true);
    const includeSchema = result.tools[0].inputSchema.properties.include;
    assert.equal(includeSchema.items.type, "string");
    assert.match(includeSchema.description, /selectors/);
    assert.match(includeSchema.description, /arguments/);
    assert.match(includeSchema.description, /stateMutability/);
    assert.doesNotMatch(includeSchema.description, /(?:^|, )functions(?:,|\.)/);
    assert.equal(transport.stderr.read(), null);
  });
});

test("returns canonical structured content and JSON fallback", async () => {
  await withClient(async (client) => {
    const result = await client.callTool({
      name: "analyze_evm_bytecode",
      arguments: {
        bytecode: await fixture("functions-runtime.hex"),
        include: ["selectors", "arguments", "stateMutability"],
      },
    });
    assert.equal(result.isError, undefined);
    assert.equal(result.structuredContent.schemaVersion, 1);
    assert.deepEqual(JSON.parse(result.content[0].text), result.structuredContent);
  });
});

test("is semantically identical to the CLI", async () => {
  const bytecode = await fixture("functions-runtime.hex");
  const cliResult = spawnSync(
    process.execPath,
    [
      cli,
      "analyze",
      "--bytecode",
      bytecode,
      "--include",
      "selectors,arguments,stateMutability,metadata",
    ],
    { encoding: "utf8", env: childEnvironment },
  );
  assert.equal(cliResult.status, 0, cliResult.stderr || String(cliResult.error));
  const expected = JSON.parse(cliResult.stdout);
  await withClient(async (client) => {
    const result = await client.callTool({
      name: "analyze_evm_bytecode",
      arguments: {
        bytecode,
        include: ["selectors", "arguments", "stateMutability", "metadata"],
      },
    });
    assert.deepEqual(result.structuredContent, expected);
  });
});

test("returns canonical errors for semantically invalid requests", async () => {
  await withClient(async (client) => {
    const cases = [
      [{ bytecode: "" }, "INVALID_BYTECODE"],
      [{ bytecode: "0xzz" }, "INVALID_BYTECODE"],
      [
        { bytecode: `0x${"00".repeat(1024 * 1024 + 1)}` },
        "BYTECODE_TOO_LARGE",
      ],
      [{ bytecode: "00", offset: -1 }, "INVALID_RANGE"],
      [{ bytecode: "00", limit: 5001 }, "INVALID_RANGE"],
      [{ bytecode: "00", include: ["unknown"] }, "UNKNOWN_SECTION"],
      [
        { bytecode: "00", include: ["selectors", "selectors"] },
        "INVALID_REQUEST",
      ],
    ];

    for (const [arguments_, code] of cases) {
      const result = await client.callTool({
        name: "analyze_evm_bytecode",
        arguments: arguments_,
      });
      assert.equal(result.isError, true);
      assert.equal(result.structuredContent.schemaVersion, 1);
      assert.equal(result.structuredContent.error.code, code);
      assert.deepEqual(
        JSON.parse(result.content[0].text),
        result.structuredContent,
      );
    }
  });
});

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import Ajv from "ajv";

import {
  analyzeBytecode,
  normalizeAgentError,
} from "../../../javascript/dist/agent_api.mjs";
import { createEvmoleServer } from "../src/server.mjs";

test("creates one MCP server without writing stdout or stderr", () => {
  const stdout = process.stdout.write;
  const stderr = process.stderr.write;
  let writes = 0;
  process.stdout.write = () => {
    writes += 1;
    return true;
  };
  process.stderr.write = () => {
    writes += 1;
    return true;
  };
  try {
    const server = createEvmoleServer({ analyzeBytecode, normalizeAgentError });
    assert.ok(server);
  } finally {
    process.stdout.write = stdout;
    process.stderr.write = stderr;
  }
  assert.equal(writes, 0);
});

test("server registry metadata is current-schema valid and stdio-only", async () => {
  const manifest = JSON.parse(
    await readFile(new URL("../server.json", import.meta.url), "utf8"),
  );
  const schema = JSON.parse(
    await readFile("/tmp/mcp-server.schema.json", "utf8").catch(() =>
      readFile(new URL("./server-schema-fallback.json", import.meta.url), "utf8")),
  );
  const ajv = new Ajv({ strict: false, formats: { uri: true } });
  const validate = ajv.compile(schema);
  assert.equal(validate(manifest), true, JSON.stringify(validate.errors));
  assert.equal(manifest.name, "io.github.cdump/evmole");
  assert.equal(manifest.repository.subfolder, "agent/mcp");
  assert.equal(manifest.remotes, undefined);
  assert.deepEqual(manifest.packages.map((entry) => entry.transport.type), ["stdio"]);
  assert.ok(manifest.packages.every((entry) => entry.environmentVariables === undefined));
});

test("runtime source contains no network or remote transport implementation", async () => {
  const source = await readFile(new URL("../src/server.mjs", import.meta.url), "utf8");
  assert.doesNotMatch(source, /https?:|fetch\(|StreamableHTTP|SSEServer|listen\(/);
});

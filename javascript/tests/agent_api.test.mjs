import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import Ajv2020 from "ajv/dist/2020.js";

import {
  AGENT_SCHEMA_VERSION,
  AgentApiError,
  EVMOLE_VERSION,
  MAX_ANALYSIS_LIMIT,
  MAX_BYTECODE_BYTES,
  analyzeBytecode,
  normalizeAgentError,
  validateAnalysisRequest,
} from "../dist/agent_api.mjs";
import { contractInfo } from "../dist/evmole_node.mjs";

const fixture = async (name) =>
  (await readFile(new URL(`../../agent/tests/fixtures/${name}`, import.meta.url), "utf8")).trim();
const schema = async (name) =>
  JSON.parse(
    await readFile(
      new URL(`../../agent/schemas/${name}-v1.schema.json`, import.meta.url),
      "utf8",
    ),
  );

test("normalizes bytecode and defaults", () => {
  const lower = validateAnalysisRequest({ bytecode: "AaBb" });
  assert.equal(lower.bytecode, "0xaabb");
  assert.deepEqual(lower.include, [
    "selectors",
    "arguments",
    "stateMutability",
    "storage",
    "transientStorage",
    "metadata",
  ]);
  assert.equal(lower.offset, 0);
  assert.equal(lower.limit, 1000);
  assert.equal(
    validateAnalysisRequest({ bytecode: "0XAABB" }).bytecode,
    "0xaabb",
  );
});

test("returns stable validation errors", () => {
  const invalid = [
    [{}, "INVALID_REQUEST"],
    [{ bytecode: "" }, "INVALID_BYTECODE"],
    [{ bytecode: "0x0" }, "INVALID_BYTECODE"],
    [{ bytecode: "0xzz" }, "INVALID_BYTECODE"],
    [{ bytecode: "00", include: ["unknown"] }, "UNKNOWN_SECTION"],
    [{ bytecode: "00", include: ["functions"] }, "UNKNOWN_SECTION"],
    [{ bytecode: "00", offset: -1 }, "INVALID_RANGE"],
    [{ bytecode: "00", limit: MAX_ANALYSIS_LIMIT + 1 }, "INVALID_RANGE"],
    [{ bytecode: "00", extra: true }, "INVALID_REQUEST"],
  ];
  for (const [request, code] of invalid) {
    assert.throws(
      () => validateAnalysisRequest(request),
      (error) => error instanceof AgentApiError && error.code === code,
    );
  }
  const envelope = normalizeAgentError(new Error("private /tmp/path"));
  assert.equal(envelope.error.code, "INTERNAL_ERROR");
  assert.doesNotMatch(JSON.stringify(envelope), /private|\/tmp/);
});

test("rejects oversized input before analysis", () => {
  const bytecode = `0x${"00".repeat(MAX_BYTECODE_BYTES + 1)}`;
  assert.throws(
    () => analyzeBytecode({ bytecode }),
    (error) => error.code === "BYTECODE_TOO_LARGE",
  );
});

test("produces deterministic schema-valid responses", async () => {
  const bytecode = await fixture("functions-runtime.hex");
  const first = analyzeBytecode({
    bytecode,
    include: [
      "selectors",
      "arguments",
      "stateMutability",
      "storage",
      "basicBlocks",
      "controlFlowGraph",
    ],
  });
  const second = analyzeBytecode({
    bytecode: bytecode.slice(2).toUpperCase(),
    include: [
      "selectors",
      "arguments",
      "stateMutability",
      "storage",
      "basicBlocks",
      "controlFlowGraph",
    ],
  });
  assert.deepEqual(first, second);
  assert.equal(first.schemaVersion, AGENT_SCHEMA_VERSION);
  assert.equal(first.input.byteLength, (bytecode.length - 2) / 2);
  assert.match(first.input.normalizedBytecodeHash, /^sha256:[0-9a-f]{64}$/);
  assert.ok(first.analysis.functions.every((fn) => /^0x[0-9a-f]{8}$/.test(fn.selector)));
  assert.equal(first.analysis.metadata, null);
  assert.equal(first.analysis.disassembly, null);

  const ajv = new Ajv2020({ strict: false });
  const validate = ajv.compile(await schema("response"));
  assert.equal(validate(first), true, JSON.stringify(validate.errors));
});

test("canonical request and error schemas accept adapter documents", async () => {
  const ajv = new Ajv2020({ strict: false });
  const validateRequest = ajv.compile(await schema("request"));
  const validateError = ajv.compile(await schema("error"));
  const request = {
    bytecode: await fixture("minimal-runtime.hex"),
    include: ["selectors"],
    offset: 0,
    limit: 1,
  };
  const error = normalizeAgentError(
    new AgentApiError("INVALID_BYTECODE", "Bytecode is malformed.", {
      field: "bytecode",
    }),
  );
  assert.equal(validateRequest(request), true, JSON.stringify(validateRequest.errors));
  assert.equal(validateError(error), true, JSON.stringify(validateError.errors));

  const oversizedUnprefixed = {
    bytecode: "00".repeat(MAX_BYTECODE_BYTES + 1),
  };
  assert.equal(validateRequest(oversizedUnprefixed), false);
  assert.equal(
    validateRequest({
      bytecode: "00",
      offset: Number.MAX_SAFE_INTEGER + 1,
    }),
    false,
  );
});

test("matches checked-in canonical fixtures", async () => {
  for (const [expectedName, fixtureName, include] of [
    ["summary", "minimal-runtime.hex", undefined],
    [
      "functions",
      "functions-runtime.hex",
      ["selectors", "arguments", "stateMutability"],
    ],
    ["storage", "storage-runtime.hex", ["storage", "transientStorage"]],
    ["metadata", "metadata-runtime.hex", ["metadata"]],
  ]) {
    const expected = JSON.parse(
      await readFile(
        new URL(`../../agent/tests/expected/${expectedName}.json`, import.meta.url),
        "utf8",
      ),
    );
    expected.evmoleVersion = EVMOLE_VERSION;
    const request = { bytecode: await fixture(fixtureName) };
    if (include) request.include = include;
    assert.deepEqual(analyzeBytecode(request), expected);
  }
});

test("distinguishes requested empty and unrequested sections", async () => {
  const bytecode = await fixture("minimal-runtime.hex");
  const response = analyzeBytecode({ bytecode, include: ["selectors"] });
  assert.deepEqual(response.analysis.functions, []);
  assert.equal(response.analysis.storage, null);
  assert.deepEqual(response.pagination.functions, {
    offset: 0,
    limit: 1000,
    returned: 0,
    available: 0,
    truncated: false,
  });
});

test("keeps selector, argument, and mutability analysis granular", async () => {
  const bytecode = await fixture("functions-runtime.hex");
  const selectors = analyzeBytecode({ bytecode, include: ["selectors"] });
  const argumentsOnly = analyzeBytecode({ bytecode, include: ["arguments"] });
  const mutabilityOnly = analyzeBytecode({
    bytecode,
    include: ["stateMutability"],
  });
  const selectorsWithStorage = analyzeBytecode({
    bytecode,
    include: ["selectors", "storage"],
  });

  assert.deepEqual(Object.keys(selectors.analysis.functions[0]), [
    "selector",
    "bytecodeOffset",
    "dispatch",
  ]);
  assert.equal(argumentsOnly.analysis.functions[0].arguments, "");
  assert.equal(
    Object.hasOwn(argumentsOnly.analysis.functions[0], "stateMutability"),
    false,
  );
  assert.equal(mutabilityOnly.analysis.functions[0].stateMutability, "payable");
  assert.equal(
    Object.hasOwn(mutabilityOnly.analysis.functions[0], "arguments"),
    false,
  );
  assert.equal(
    Object.hasOwn(selectorsWithStorage.analysis.functions[0], "arguments"),
    false,
  );
  assert.deepEqual(selectors.warnings, []);
  assert.match(argumentsOnly.warnings[0], /^Arguments are inferred/);
  assert.match(mutabilityOnly.warnings[0], /^State mutability is inferred/);
});

test("paginates every requested list section and reports truncation", async () => {
  const bytecode = await fixture("control-flow-runtime.hex");
  const full = analyzeBytecode({
    bytecode,
    include: ["disassembly", "basicBlocks", "controlFlowGraph"],
  });
  const paged = analyzeBytecode({
    bytecode,
    include: ["disassembly", "basicBlocks", "controlFlowGraph"],
    offset: 1,
    limit: 1,
  });
  assert.equal(paged.analysis.disassembly.length, 1);
  assert.equal(paged.analysis.basicBlocks.length, 1);
  assert.equal(paged.analysis.controlFlowGraph.blocks.length, 1);
  assert.equal(paged.pagination.disassembly.available, full.analysis.disassembly.length);
  assert.equal(paged.pagination.disassembly.truncated, true);
  assert.match(paged.warnings.join(" "), /truncated/);
});

test("does not report truncation when a requested section is empty", async () => {
  const response = analyzeBytecode({
    bytecode: await fixture("minimal-runtime.hex"),
    include: ["selectors"],
    offset: 1,
  });
  assert.deepEqual(response.pagination.functions, {
    offset: 1,
    limit: 1000,
    returned: 0,
    available: 0,
    truncated: false,
  });
  assert.deepEqual(response.warnings, []);
});

test("reports truncation for a page beyond existing records", async () => {
  const response = analyzeBytecode({
    bytecode: await fixture("functions-runtime.hex"),
    include: ["selectors"],
    offset: 100,
  });
  assert.ok(response.pagination.functions.available > 0);
  assert.equal(response.pagination.functions.returned, 0);
  assert.equal(response.pagination.functions.truncated, true);
  assert.match(response.warnings[0], /functions is truncated/);
});

test("metadata is normalized and paginated", async () => {
  const response = analyzeBytecode({
    bytecode: await fixture("metadata-runtime.hex"),
    include: ["metadata"],
  });
  assert.equal(response.analysis.metadata.entries[0].key, "solc");
  assert.equal(response.pagination.metadata.available, 1);
});

test("keeps persistent and transient storage domains distinct", async () => {
  const response = analyzeBytecode({
    bytecode: await fixture("transient-storage-runtime.hex"),
    include: ["storage", "transientStorage"],
  });
  assert.deepEqual(response.analysis.storage, []);
  assert.equal(response.analysis.transientStorage.length, 1);
  assert.deepEqual(response.analysis.transientStorage[0].writes, ["0x11223344"]);
});

test("preserves CBOR integers beyond JSON safe precision as encoded strings", () => {
  const response = analyzeBytecode({
    bytecode: "0x00a161691b0020000000000000000c",
    include: ["metadata"],
  });
  assert.deepEqual(response.analysis.metadata.entries[0].value, {
    type: "undecoded",
    value: "1b0020000000000000",
  });
});

test("matches the direct binding without duplicate analysis mapping", async () => {
  const bytecode = await fixture("functions-runtime.hex");
  const direct = contractInfo(bytecode, {
    selectors: true,
    arguments: true,
    stateMutability: true,
  });
  const response = analyzeBytecode({
    bytecode,
    include: ["selectors", "arguments", "stateMutability"],
  });
  assert.deepEqual(
    response.analysis.functions.map(({ selector, ...entry }) => ({
      selector: selector.slice(2),
      ...entry,
    })),
    direct.functions,
  );
});

test("adapter does not write stdout or stderr", async () => {
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
    analyzeBytecode({
      bytecode: await fixture("minimal-runtime.hex"),
      include: [],
    });
  } finally {
    process.stdout.write = stdout;
    process.stderr.write = stderr;
  }
  assert.equal(writes, 0);
});

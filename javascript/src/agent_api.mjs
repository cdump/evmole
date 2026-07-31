import { createHash } from "node:crypto";
import { createRequire } from "node:module";

import { contractInfo } from "./evmole_node.mjs";

const { version: EVMOLE_VERSION } =
  createRequire(import.meta.url)("../package.json");

export { EVMOLE_VERSION };

export const AGENT_SCHEMA_VERSION = 1;
export const MAX_BYTECODE_BYTES = 1024 * 1024;
export const DEFAULT_ANALYSIS_LIMIT = 1000;
export const MAX_ANALYSIS_LIMIT = 5000;
export const ANALYSIS_SECTIONS = Object.freeze([
  "selectors",
  "arguments",
  "stateMutability",
  "storage",
  "transientStorage",
  "metadata",
  "disassembly",
  "basicBlocks",
  "controlFlowGraph",
]);
export const DEFAULT_ANALYSIS_SECTIONS = Object.freeze([
  "selectors",
  "arguments",
  "stateMutability",
  "storage",
  "transientStorage",
  "metadata",
]);

const SECTION_SET = new Set(ANALYSIS_SECTIONS);

export class AgentApiError extends Error {
  constructor(code, message, details) {
    super(message);
    this.name = "AgentApiError";
    this.code = code;
    this.details = details;
  }

  toJSON() {
    return createErrorEnvelope(this.code, this.message, this.details);
  }
}

export function createErrorEnvelope(code, message, details) {
  const error = { code, message };
  if (details && Object.keys(details).length > 0) {
    error.details = details;
  }
  return {
    schemaVersion: AGENT_SCHEMA_VERSION,
    error,
  };
}

export function normalizeAgentError(error) {
  if (error instanceof AgentApiError) {
    return error.toJSON();
  }
  return createErrorEnvelope(
    "INTERNAL_ERROR",
    "EVMole could not complete the analysis.",
  );
}

export function validateAnalysisRequest(request) {
  if (!request || typeof request !== "object" || Array.isArray(request)) {
    throw new AgentApiError(
      "INVALID_REQUEST",
      "The analysis request must be a JSON object.",
    );
  }

  const allowedKeys = new Set(["bytecode", "include", "offset", "limit"]);
  const unknownKeys = Object.keys(request).filter((key) => !allowedKeys.has(key));
  if (unknownKeys.length > 0) {
    throw new AgentApiError(
      "INVALID_REQUEST",
      `Unknown request field: ${unknownKeys[0]}.`,
      { field: unknownKeys[0] },
    );
  }

  const bytecode = normalizeBytecode(request.bytecode);
  const include = normalizeInclude(request.include);
  const offset = normalizeInteger(request.offset, "offset", 0, 0, Number.MAX_SAFE_INTEGER);
  const limit = normalizeInteger(
    request.limit,
    "limit",
    DEFAULT_ANALYSIS_LIMIT,
    1,
    MAX_ANALYSIS_LIMIT,
  );

  return { bytecode, include, offset, limit };
}

export function analyzeBytecode(request) {
  const validated = validateAnalysisRequest(request);
  const options = analysisOptions(validated.include);
  let raw;
  try {
    raw = contractInfo(validated.bytecode, options);
  } catch {
    throw new AgentApiError(
      "ANALYSIS_FAILED",
      "EVMole could not analyze the supplied runtime bytecode.",
    );
  }

  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new AgentApiError(
      "ANALYSIS_FAILED",
      "EVMole returned an invalid analysis result.",
    );
  }

  const analysis = {
    functions: null,
    storage: null,
    transientStorage: null,
    metadata: null,
    disassembly: null,
    basicBlocks: null,
    controlFlowGraph: null,
  };
  const pagination = {};
  const warnings = [];
  const requested = new Set(validated.include);
  const functionsRequested =
    requested.has("selectors") ||
    requested.has("arguments") ||
    requested.has("stateMutability");

  if (functionsRequested) {
    const functions = normalizeFunctions(raw.functions ?? [], {
      arguments: requested.has("arguments"),
      stateMutability: requested.has("stateMutability"),
    });
    analysis.functions = pageSection(
      "functions",
      functions,
      validated,
      pagination,
      warnings,
    );
  }
  if (requested.has("storage")) {
    const storage = normalizeStorage(raw.storage ?? []);
    analysis.storage = pageSection(
      "storage",
      storage,
      validated,
      pagination,
      warnings,
    );
  }
  if (requested.has("transientStorage")) {
    const storage = normalizeStorage(raw.transientStorage ?? []);
    analysis.transientStorage = pageSection(
      "transientStorage",
      storage,
      validated,
      pagination,
      warnings,
    );
  }
  if (requested.has("metadata")) {
    if (raw.metadata) {
      const metadata = normalizeJson(raw.metadata);
      const entries = pageSection(
        "metadata",
        metadata.entries ?? [],
        validated,
        pagination,
        warnings,
      );
      analysis.metadata = { ...metadata, entries };
    } else {
      analysis.metadata = null;
      pagination.metadata = pageDescriptor(validated, 0, 0);
    }
  }
  if (requested.has("disassembly")) {
    const disassembly = normalizeJson(raw.disassembled ?? []);
    analysis.disassembly = pageSection(
      "disassembly",
      disassembly,
      validated,
      pagination,
      warnings,
    );
  }
  if (requested.has("basicBlocks")) {
    const basicBlocks = normalizeJson(raw.basicBlocks ?? []);
    analysis.basicBlocks = pageSection(
      "basicBlocks",
      basicBlocks,
      validated,
      pagination,
      warnings,
    );
  }
  if (requested.has("controlFlowGraph")) {
    const cfg = normalizeJson(raw.controlFlowGraph ?? { blocks: [] });
    cfg.blocks = [...(cfg.blocks ?? [])].sort((a, b) => a.id - b.id);
    cfg.blocks = pageSection(
      "controlFlowGraph",
      cfg.blocks,
      validated,
      pagination,
      warnings,
    );
    analysis.controlFlowGraph = cfg;
  }

  const inferenceWarning = createInferenceWarning(requested);
  if (inferenceWarning) {
    warnings.unshift(inferenceWarning);
  }

  const bytes = Buffer.from(validated.bytecode.slice(2), "hex");
  return {
    schemaVersion: AGENT_SCHEMA_VERSION,
    evmoleVersion: EVMOLE_VERSION,
    input: {
      byteLength: bytes.length,
      normalizedBytecodeHash: `sha256:${createHash("sha256").update(bytes).digest("hex")}`,
    },
    analysis,
    pagination,
    warnings: warnings.slice(0, 100),
  };
}

function normalizeBytecode(value) {
  if (typeof value !== "string") {
    throw new AgentApiError(
      "INVALID_REQUEST",
      "The bytecode field must be a hexadecimal string.",
      { field: "bytecode" },
    );
  }

  const hex = value.startsWith("0x") || value.startsWith("0X")
    ? value.slice(2)
    : value;
  if (hex.length === 0) {
    throw new AgentApiError(
      "INVALID_BYTECODE",
      "Bytecode must not be empty.",
    );
  }
  if (hex.length % 2 !== 0) {
    throw new AgentApiError(
      "INVALID_BYTECODE",
      "Bytecode must contain an even number of hexadecimal digits.",
      { length: hex.length },
    );
  }
  const badIndex = hex.search(/[^0-9a-fA-F]/);
  if (badIndex !== -1) {
    throw new AgentApiError(
      "INVALID_BYTECODE",
      "Bytecode contains a non-hexadecimal character.",
      { position: badIndex + (value.length !== hex.length ? 2 : 0) },
    );
  }
  const byteLength = hex.length / 2;
  if (byteLength > MAX_BYTECODE_BYTES) {
    throw new AgentApiError(
      "BYTECODE_TOO_LARGE",
      `Bytecode exceeds the ${MAX_BYTECODE_BYTES}-byte limit.`,
      { byteLength, maximumByteLength: MAX_BYTECODE_BYTES },
    );
  }
  return `0x${hex.toLowerCase()}`;
}

function normalizeInclude(value) {
  if (value === undefined) {
    return [...DEFAULT_ANALYSIS_SECTIONS];
  }
  if (!Array.isArray(value)) {
    throw new AgentApiError(
      "INVALID_REQUEST",
      "The include field must be an array of section names.",
      { field: "include" },
    );
  }

  const include = [];
  const seen = new Set();
  for (const section of value) {
    if (typeof section !== "string" || !SECTION_SET.has(section)) {
      throw new AgentApiError(
        "UNKNOWN_SECTION",
        `Unknown analysis section: ${String(section)}.`,
        { section, allowedSections: ANALYSIS_SECTIONS },
      );
    }
    if (seen.has(section)) {
      throw new AgentApiError(
        "INVALID_REQUEST",
        `Analysis section is repeated: ${section}.`,
        { section },
      );
    }
    seen.add(section);
    include.push(section);
  }
  return include;
}

function normalizeInteger(value, field, fallback, minimum, maximum) {
  if (value === undefined) {
    return fallback;
  }
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new AgentApiError(
      "INVALID_RANGE",
      `${field} must be an integer between ${minimum} and ${maximum}.`,
      { field, minimum, maximum },
    );
  }
  return value;
}

function analysisOptions(include) {
  const sections = new Set(include);
  return {
    selectors: sections.has("selectors"),
    arguments: sections.has("arguments"),
    stateMutability: sections.has("stateMutability"),
    storage: sections.has("storage") || sections.has("transientStorage"),
    metadata: sections.has("metadata"),
    disassemble: sections.has("disassembly"),
    basicBlocks: sections.has("basicBlocks"),
    controlFlowGraph: sections.has("controlFlowGraph"),
  };
}

function normalizeFunctions(functions, include) {
  return functions
    .map((entry) => {
      const normalized = {
        selector: normalizePrefixedHex(entry.selector),
        bytecodeOffset: entry.bytecodeOffset,
        dispatch: entry.dispatch,
      };
      if (
        include.arguments &&
        entry.arguments !== undefined &&
        entry.arguments !== null
      ) {
        normalized.arguments = entry.arguments;
      }
      if (
        include.stateMutability &&
        entry.stateMutability !== undefined &&
        entry.stateMutability !== null
      ) {
        normalized.stateMutability = entry.stateMutability;
      }
      return normalized;
    })
    .sort(
      (a, b) =>
        a.selector.localeCompare(b.selector) ||
        a.bytecodeOffset - b.bytecodeOffset,
    );
}

function createInferenceWarning(requested) {
  const fields = [];
  if (requested.has("arguments")) fields.push("arguments");
  if (requested.has("stateMutability")) fields.push("state mutability");
  if (requested.has("storage") || requested.has("transientStorage")) {
    fields.push("storage types", "storage labels");
  }
  if (fields.length === 0) return null;
  const list = fields.length === 1
    ? fields[0]
    : fields.length === 2
      ? `${fields[0]} and ${fields[1]}`
      : `${fields.slice(0, -1).join(", ")}, and ${fields.at(-1)}`;
  const verb = fields.length === 1 && fields[0] === "state mutability"
    ? "is"
    : "are";
  return `${list[0].toUpperCase()}${list.slice(1)} ${verb} inferred from runtime bytecode rather than verified from source.`;
}

function normalizeStorage(records) {
  return records
    .map((entry) => ({
      slot: normalizePrefixedHex(entry.slot),
      offset: entry.offset,
      type: entry.type,
      reads: (entry.reads ?? []).map(normalizePrefixedHex).sort(),
      writes: (entry.writes ?? []).map(normalizePrefixedHex).sort(),
    }))
    .sort((a, b) => a.slot.localeCompare(b.slot) || a.offset - b.offset);
}

function normalizePrefixedHex(value) {
  const hex = String(value).replace(/^0x/i, "").toLowerCase();
  return `0x${hex}`;
}

function normalizeJson(value) {
  if (Array.isArray(value)) {
    return value.map(normalizeJson);
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [key, normalizeJson(item)]),
    );
  }
  if (typeof value === "number" && !Number.isSafeInteger(value)) {
    return String(value);
  }
  return value;
}

function pageSection(name, values, request, pagination, warnings) {
  const available = values.length;
  const returned = values.slice(request.offset, request.offset + request.limit);
  const descriptor = pageDescriptor(request, available, returned.length);
  pagination[name] = descriptor;
  if (descriptor.truncated) {
    warnings.push(
      `${name} is truncated: returned ${descriptor.returned} of ${descriptor.available} items from offset ${descriptor.offset}.`,
    );
  }
  return returned;
}

function pageDescriptor(request, available, returned) {
  return {
    offset: request.offset,
    limit: request.limit,
    returned,
    available,
    truncated: returned < available,
  };
}

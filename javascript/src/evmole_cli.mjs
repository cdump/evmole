#!/usr/bin/env node

import { readFile, stat } from "node:fs/promises";

import {
  AgentApiError,
  EVMOLE_VERSION,
  MAX_BYTECODE_BYTES,
  analyzeBytecode,
  createErrorEnvelope,
  normalizeAgentError,
} from "./agent_api.mjs";

class CliUsageError extends Error {
  constructor(message, details) {
    super(message);
    this.name = "CliUsageError";
    this.details = details;
  }
}

const args = process.argv.slice(2);
const MAX_INPUT_CHARACTERS = 2 * MAX_BYTECODE_BYTES + 4;

try {
  const result = await main(args);
  if (result !== undefined) {
    process.stdout.write(`${result}\n`);
  }
} catch (error) {
  const envelope = error instanceof CliUsageError
    ? createErrorEnvelope("INVALID_REQUEST", error.message, error.details)
    : normalizeAgentError(error);
  process.stdout.write(`${JSON.stringify(envelope)}\n`);
  process.exitCode = exitCodeFor(error);
}

async function main(argv) {
  if (argv.length === 0 || argv[0] === "--help" || argv[0] === "-h") {
    return helpText();
  }
  if (argv[0] === "--version" || argv[0] === "-V") {
    return EVMOLE_VERSION;
  }
  if (argv[0] === "schema") {
    if (argv.length !== 1) {
      throw new CliUsageError("The schema command does not accept options.");
    }
    return JSON.stringify(await loadSchemas());
  }
  if (argv[0] !== "analyze") {
    throw new CliUsageError(`Unknown command: ${argv[0]}.`);
  }

  const options = parseAnalyzeOptions(argv.slice(1));
  if (options.help) {
    return analyzeHelpText();
  }
  const bytecode = await acquireBytecode(options);
  const request = { bytecode };
  if (options.include !== undefined) request.include = options.include;
  if (options.offset !== undefined) request.offset = options.offset;
  if (options.limit !== undefined) request.limit = options.limit;
  const response = analyzeBytecode(request);
  return JSON.stringify(response, null, options.pretty ? 2 : undefined);
}

function parseAnalyzeOptions(argv) {
  const options = {
    bytecode: undefined,
    file: undefined,
    include: undefined,
    offset: undefined,
    limit: undefined,
    pretty: false,
    help: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const option = argv[index];
    if (option === "--pretty") {
      options.pretty = true;
    } else if (option === "--help" || option === "-h") {
      options.help = true;
    } else if (option === "--bytecode" || option === "--file" || option === "--include" ||
      option === "--offset" || option === "--limit") {
      if (index + 1 >= argv.length) {
        throw new CliUsageError(`${option} requires a value.`);
      }
      const value = argv[index + 1];
      index += 1;
      if (option === "--bytecode") options.bytecode = value;
      if (option === "--file") options.file = value;
      if (option === "--include") {
        options.include = value === "" ? [] : value.split(",");
      }
      if (option === "--offset") options.offset = parseCliInteger(value, "offset");
      if (option === "--limit") options.limit = parseCliInteger(value, "limit");
    } else {
      throw new CliUsageError(`Unknown analyze option: ${option}.`);
    }
  }
  return options;
}

async function acquireBytecode(options) {
  if (options.bytecode !== undefined && options.file !== undefined) {
    throw new CliUsageError(
      "Provide either --bytecode or --file, not both.",
    );
  }
  if (options.bytecode !== undefined) {
    return options.bytecode.trim();
  }
  if (options.file !== undefined) {
    try {
      const file = await stat(options.file);
      if (file.size > MAX_INPUT_CHARACTERS) {
        throw new AgentApiError(
          "BYTECODE_TOO_LARGE",
          `Bytecode input exceeds the ${MAX_BYTECODE_BYTES}-byte limit.`,
          { inputBytes: file.size, maximumByteLength: MAX_BYTECODE_BYTES },
        );
      }
      return (await readFile(options.file, "utf8")).trim();
    } catch (error) {
      if (error instanceof AgentApiError) throw error;
      throw new CliUsageError("The bytecode file could not be read.", {
        file: options.file,
      });
    }
  }
  const stdin = process.stdin.isTTY ? "" : await readStdin();
  if (stdin.trim() === "") {
    throw new CliUsageError(
      "No bytecode input was provided. Use --bytecode, --file, or piped stdin.",
    );
  }
  return stdin.trim();
}

async function readStdin() {
  const chunks = [];
  let bytes = 0;
  for await (const chunk of process.stdin) {
    const buffer = Buffer.from(chunk);
    bytes += buffer.length;
    if (bytes > MAX_INPUT_CHARACTERS) {
      throw new AgentApiError(
        "BYTECODE_TOO_LARGE",
        `Bytecode input exceeds the ${MAX_BYTECODE_BYTES}-byte limit.`,
        { inputBytes: bytes, maximumByteLength: MAX_BYTECODE_BYTES },
      );
    }
    chunks.push(buffer);
  }
  return Buffer.concat(chunks).toString("utf8");
}

function parseCliInteger(value, field) {
  if (!/^(0|[1-9][0-9]*)$/.test(value)) {
    throw new CliUsageError(`${field} must be a non-negative integer.`);
  }
  const number = Number(value);
  if (!Number.isSafeInteger(number)) {
    throw new CliUsageError(`${field} is outside the supported integer range.`);
  }
  return number;
}

async function loadSchemas() {
  const names = ["request", "response", "error"];
  const entries = await Promise.all(
    names.map(async (name) => {
      const url = new URL(`./schemas/${name}-v1.schema.json`, import.meta.url);
      return [name, JSON.parse(await readFile(url, "utf8"))];
    }),
  );
  return { schemaVersion: 1, ...Object.fromEntries(entries) };
}

function exitCodeFor(error) {
  if (error instanceof CliUsageError) return 2;
  if (error instanceof AgentApiError) {
    if (error.code === "BYTECODE_TOO_LARGE") return 4;
    if (
      error.code === "INVALID_REQUEST" ||
      error.code === "INVALID_BYTECODE" ||
      error.code === "UNKNOWN_SECTION" ||
      error.code === "INVALID_RANGE"
    ) {
      return 3;
    }
  }
  return 1;
}

function helpText() {
  return `Usage: evmole <command>

Commands:
  analyze     Analyze deployed EVM runtime bytecode as JSON
  schema      Print the version 1 request, response, and error schemas

Options:
  -h, --help      Show help
  -V, --version   Show the EVMole package version`;
}

function analyzeHelpText() {
  return `Usage: evmole analyze [options]

Use an explicit input option, or omit both to read piped stdin:
  --bytecode <hex>   Runtime bytecode
  --file <path>      Read runtime bytecode from a local file
  piped stdin        Read runtime bytecode from standard input

Options:
  --include <names>  Comma-separated analysis features
  --offset <number>  Pagination offset (default: 0)
  --limit <number>   Pagination limit (default: 1000, maximum: 5000)
  --pretty           Pretty-print JSON
  -h, --help         Show this help`;
}

#!/usr/bin/env node

import { realpathSync } from "node:fs";
import { createRequire } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

const SERVER_NAME = "evmole-mcp";
const { version: SERVER_VERSION } =
  createRequire(import.meta.url)("../package.json");
const TOOL_DESCRIPTION =
  "Analyze deployed EVM runtime bytecode locally with EVMole. Extract function " +
  "selectors, inferred arguments and mutability, persistent or transient " +
  "storage access, metadata, disassembly, basic blocks, or a control-flow graph. " +
  "This tool does not fetch contracts, decode calldata, verify source code, or " +
  "fully decompile a contract.";

const sections = [
  "selectors",
  "arguments",
  "stateMutability",
  "storage",
  "transientStorage",
  "metadata",
  "disassembly",
  "basicBlocks",
  "controlFlowGraph",
];

export function createEvmoleServer({ analyzeBytecode, normalizeAgentError }) {
  if (typeof analyzeBytecode !== "function" || typeof normalizeAgentError !== "function") {
    throw new TypeError("EVMole adapter functions are required.");
  }

  const server = new McpServer({
    name: SERVER_NAME,
    version: SERVER_VERSION,
  });

  server.registerTool(
    "analyze_evm_bytecode",
    {
      title: "Analyze EVM bytecode",
      description: TOOL_DESCRIPTION,
      inputSchema: {
        bytecode: z
          .string()
          .describe(
            "Deployed EVM runtime bytecode as hexadecimal; maximum 1 MiB.",
          ),
        include: z
          .array(z.string())
          .optional()
          .describe(
            `Analysis features to include: ${sections.join(", ")}. ` +
            "arguments and stateMutability imply selectors.",
          ),
        offset: z.number().optional(),
        limit: z.number().optional(),
      },
    },
    async (request) => {
      try {
        const response = analyzeBytecode(request);
        return {
          content: [{ type: "text", text: JSON.stringify(response) }],
          structuredContent: response,
        };
      } catch (error) {
        const envelope = normalizeAgentError(error);
        return {
          content: [{ type: "text", text: JSON.stringify(envelope) }],
          structuredContent: envelope,
          isError: true,
        };
      }
    },
  );

  return server;
}

export async function runStdioServer(adapter) {
  const server = createEvmoleServer(adapter);
  const transport = new StdioServerTransport();
  let closing = false;

  const close = async () => {
    if (closing) return;
    closing = true;
    try {
      await server.close();
    } catch {
      // The transport may already be closed by the parent MCP client.
    }
  };

  process.once("SIGINT", () => {
    void close().finally(() => {
      process.exitCode = 0;
    });
  });
  process.once("SIGTERM", () => {
    void close().finally(() => {
      process.exitCode = 0;
    });
  });
  process.stdin.once("end", () => {
    void close();
  });

  await server.connect(transport);
}

function isMainModule() {
  if (!process.argv[1]) return false;
  try {
    return pathToFileURL(realpathSync(process.argv[1])).href === import.meta.url;
  } catch {
    return fileURLToPath(import.meta.url) === process.argv[1];
  }
}

if (isMainModule()) {
  try {
    const { analyzeBytecode, normalizeAgentError } = await import("evmole/agent");
    await runStdioServer({ analyzeBytecode, normalizeAgentError });
  } catch {
    process.stderr.write("evmole-mcp failed to start.\n");
    process.exitCode = 1;
  }
}

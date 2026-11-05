#!/usr/bin/env node

import {
  ANALYSIS_SECTIONS,
  analyzeBytecode,
  normalizeAgentError,
} from "../../../javascript/dist/agent_api.mjs";
import { runStdioServer } from "../src/server.mjs";

await runStdioServer({
  analyzeBytecode,
  normalizeAgentError,
  analysisSections: ANALYSIS_SECTIONS,
});

import fs from "node:fs";
import { initSync } from "../dist/evmole.js";

const wasm = new URL("../dist/evmole_bg.wasm", import.meta.url);
initSync(fs.readFileSync(wasm));

export {functionSelectors, functionArguments} from "../dist/evmole.js";

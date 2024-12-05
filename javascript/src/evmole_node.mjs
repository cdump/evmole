import { initSync } from "../dist/evmole.js";
import fs from "node:fs";

const path = new URL("../dist/evmole_bg.wasm", import.meta.url);
const bytes = fs.readFileSync(path);

initSync({module: bytes});

export {contractInfo, functionSelectors, functionArguments, functionStateMutability} from "../dist/evmole.js";

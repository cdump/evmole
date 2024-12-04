import { initSync } from "../dist/evmole.js";

const path = require("path").join(__dirname, "evmole_bg.wasm");
const bytes = require("fs").readFileSync(path);

initSync({module: bytes});

export {contractInfo, functionSelectors, functionArguments, functionStateMutability} from "../dist/evmole.js";

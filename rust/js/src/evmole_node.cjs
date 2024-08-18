export {functionSelectors, functionArguments} from "../dist/evmole.js";
import { initSync } from "../dist/evmole.js";

const path = require("path").join(__dirname, "evmole_bg.wasm");
const bytes = require("fs").readFileSync(path);

initSync(bytes);

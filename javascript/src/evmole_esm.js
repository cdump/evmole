export {functionSelectors, functionArguments, functionStateMutability} from "../dist/evmole.js";
import initEvmole from "../dist/evmole.js";

await initEvmole({module_or_path: new URL('evmole_bg.wasm', import.meta.url)})

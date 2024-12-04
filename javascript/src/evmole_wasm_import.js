export {contractInfo, functionSelectors, functionArguments, functionStateMutability} from "../dist/evmole.js";
import initEvmole from "../dist/evmole.js";

import wasmUrl from "../dist/evmole_bg.wasm";

await initEvmole({module_or_path: new URL(wasmUrl, import.meta.url)});

export {functionSelectors, functionArguments} from "../dist/evmole.js";
import initEvmole from "../dist/evmole.js";

import wasmUrl from "../dist/evmole_bg.wasm?url";

await initEvmole(new URL(wasmUrl, import.meta.url));

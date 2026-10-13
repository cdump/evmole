// Type-checks the package root as a CommonJS consumer; see `make test`.
import { contractInfo, type Contract, type ContractFunction } from "evmole";
// @ts-expect-error the package root has no wasm init export
import { initSync } from "evmole";

const info: Contract = contractInfo("00", { selectors: true, arguments: true });
const functions: ContractFunction[] | undefined = info.functions;
// @ts-expect-error functions is optional
const count: number = info.functions.length;

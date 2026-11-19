// Type-checks the package root as a CommonJS consumer; see `make test`.
import { contractInfo, type Block, type Contract, type ContractFunction, type ProxySource, type DynamicJump } from "evmole";
// @ts-expect-error the package root has no wasm init export
import { initSync } from "evmole";

const info: Contract = contractInfo("00", { selectors: true, arguments: true });
const functions: ContractFunction[] | undefined = info.functions;
// @ts-expect-error functions is optional
const count: number = info.functions.length;

const blocks: Block[] = contractInfo("00", { controlFlowGraph: true }).controlFlowGraph?.blocks ?? [];
for (const block of blocks) {
  if (block.type === "DynamicJump") {
    const targets: DynamicJump[] = block.data.to;
  } else if (block.type === "DynamicJumpi") {
    const targets: DynamicJump[] = block.data.true_to;
    const fallthrough: number = block.data.false_to;
  }
}

const sources: ProxySource[] | undefined = contractInfo("00", { proxy: true }).proxy;
for (const source of sources ?? []) {
  if (source.type === "Call" && source.data.address.type === "Storage") {
    const slot: string = source.data.address.data.slot;
  }
}

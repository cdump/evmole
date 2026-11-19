# Interpretation reference

## Runtime and creation bytecode

EVMole schema version 2 accepts deployed/runtime bytecode: the code stored at a
contract address and executed for calls. Creation bytecode is constructor code
that may copy, transform, or return runtime code. EVMole does not execute a
constructor or automatically locate its returned runtime. Fetch or obtain the
deployed code first.

## Evidence and inference

- A selector is a four-byte dispatch value found by static bytecode analysis.
  It does not supply a verified function name.
- Argument types are likely ABI types inferred from calldata use. Optimizations,
  unusual dispatch, assembly, proxy patterns, or unreachable paths can make
  them incomplete or ambiguous.
- State mutability is inferred from reachable operations and call-value
  handling. It is not a source ABI declaration.
- Storage records describe observed persistent or transient access and inferred
  types. They are not a verified source layout, variable name, inheritance
  layout, or complete description of every possible dynamic slot.
- Metadata is decoded only when a valid terminal CBOR trailer exists.
- `Address` gives the fixed implementation address in the runtime. `Eip7702`
  gives the delegate address in the account's delegation designator.
  `Storage` identifies the bytes to read (`size` bytes, `offset` bytes from
  the slot's low end). `Call` describes a beacon or registry lookup.
  `PerSelector` means the target depends on the incoming selector, as in a
  diamond. Several sources mean the active one can depend on the caller
  or contract state. An empty array means no source was found, not that the
  contract never delegates. An implementation contract, such as a UUPS
  implementation or a diamond facet, is not itself a proxy.
- Disassembly and control-flow output describe bytecode structure, not exact
  high-level source semantics.

Never resolve selectors to names without a separately authorized signature
source. Do not claim that missing evidence proves an operation is impossible.

## Pagination

Unrequested sections are `null`. Requested list-like sections are arrays,
including empty arrays. The `pagination` entry reports `offset`, `limit`,
`returned`, `available`, and `truncated`. Metadata pagination applies to its
entries; control-flow pagination applies to its blocks. Mention truncation when
it affects the answer and request the next page only if it is useful.

## Safe wording

Prefer:

- "The runtime contains selector `0x12345678`; EVMole infers one `address`
  argument."
- "The analyzed paths appear to read slot `0x00`."
- "EVMole infers `view`; this is not a verified ABI declaration."
- "The fallback appears to delegate to the address in slot `0x3608…2bbc`;
  its current value needs chain state."

Avoid:

- "The function is definitely `transfer(address)`."
- "This is the complete Solidity storage layout."
- "The original source contained this exact control structure."

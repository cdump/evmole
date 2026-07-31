# Interpretation reference

## Runtime and creation bytecode

EVMole schema version 1 accepts deployed/runtime bytecode: the code stored at a
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

Avoid:

- "The function is definitely `transfer(address)`."
- "This is the complete Solidity storage layout."
- "The original source contained this exact control structure."

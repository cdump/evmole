# EVMole Python

EVMole Python is built with [PyO3](https://pyo3.rs/) for different operating systems and Python versions. In most cases, pip will install a pre-built wheel. However, the source distribution (sdist) is also published, allowing installation on other architectures and Python versions (this is automatically tested).

## Installation
To install or upgrade EVMole, use pip:
```bash
$ pip install evmole --upgrade
```

<!-- manually generated with `pydoc-markdown -I .`, ln -s 'evmole.pyi' to 'evmole.py' -->
## API

### contract\_info

```python
def contract_info(code: Union[bytes, str],
                  *,
                  selectors: bool = False,
                  arguments: bool = False,
                  state_mutability: bool = False,
                  storage: bool = False,
                  disassemble: bool = False,
                  basic_blocks: bool = False,
                  control_flow_graph: bool = False) -> Contract
```

Extracts information about a smart contract from its EVM bytecode.

**Arguments**:

- `code` - Runtime bytecode as a hex string (with or without '0x' prefix)
  or raw bytes.
- `selectors` - When True, extracts function selectors.
- `arguments` - When True, extracts function arguments.
- `state_mutability` - When True, extracts function state mutability.
- `storage` - When True, extracts the contract's storage layout.
- `disassemble` - When True, includes disassembled bytecode.
- `basic_blocks` - When True, extracts basic block ranges.
- `control_flow_graph` - When True, builds control flow graph.

**Returns**:

- `Contract` - Object containing the requested smart contract information. Fields that
  weren't requested to be extracted will be None.

### Contract

```python
class Contract():
    functions: Optional[List[Function]]
    storage: Optional[List[StorageRecord]]
    disassembled: Optional[List[Tuple[int, str]]]
    basic_blocks: Optional[List[Tuple[int, int]]]
    control_flow_graph: Optional[ControlFlowGraph]
```

Contains analyzed information about a smart contract.

**Attributes**:

- `functions` - List of detected contract functions. None if no functions were extracted
- `storage` - List of contract storage records. None if storage layout was not extracted
- `disassembled` - List of bytecode instructions, where each element is [offset, instruction]. None if disassembly was not requested
- `basic_blocks` - List of basic block ranges as (first_op, last_op) offsets. None if basic blocks were not requested
- `control_flow_graph` - Control flow graph of the contract. None if control flow analysis was not requested

### Function

```python
class Function():
    selector: str
    bytecode_offset: int
    arguments: Optional[str]
    state_mutability: Optional[str]
```

Represents a public smart contract function.

**Attributes**:

- `selector` - Function selector as a 4-byte hex string without '0x' prefix (e.g., 'aabbccdd').
- `bytecode_offset` - Starting byte offset within the EVM bytecode for the function body.
- `arguments` - Function argument types in canonical format (e.g., 'uint256,address[]').
  None if arguments were not extracted
- `state_mutability` - Function's state mutability ('pure', 'view', 'payable', or 'nonpayable').
  None if state mutability was not extracted

### StorageRecord

```python
class StorageRecord():
    slot: str
    offset: int
    type: str
    reads: List[str]
    writes: List[str]
```

Represents a storage variable record in a smart contract's storage layout.

**Attributes**:

- `slot` - Storage slot number as a hex string (e.g., '0', '1b').
- `offset` - Byte offset within the storage slot (0-31).
- `type` - Variable type (e.g., 'uint256', 'mapping(address => uint256)', 'bytes32').
- `reads` - List of function selectors that read from this storage location.
- `writes` - List of function selectors that write to this storage location.

### ControlFlowGraph

```python
class ControlFlowGraph():
    blocks: List[Block]
```

Represents the control flow graph of the contract bytecode.

**Attributes**:

- `blocks` - List of basic blocks in the control flow graph

### Block

```python
class Block():
    start: int
    end: int
    btype: BlockType
```

Represents a basic block in the control flow graph.

**Attributes**:

- `start` - Byte offset where the block's first opcode begins
- `end` - Byte offset where the block's last opcode begins
- `btype` - Type of the block and its control flow


### BlockType

```python
class BlockType():
    class Terminate:
        success: bool

    class Jump:
        to: int

    class Jumpi:
        true_to: int
        false_to: int

    class DynamicJump:
        to: List[DynamicJump]

    class DynamicJumpi:
        true_to: List[DynamicJump]
        false_to: int
```

Represents the type of a basic block and its control flow.

This is an enum-like class, all child classes are derived from `BlockType` class

#### Terminate
Block terminates execution
- `success` - True for normal termination (STOP/RETURN), False for REVERT/INVALID


#### Jump
Block ends with unconditional jump
- `to` -  Destination basic block offset

#### Jumpi
Block ends with conditional jump
- `true_to` - Destination if condition is true
- `false_to` - Destination if condition is false (fall-through)

#### DynamicJump
Block ends with jump to computed destination
- `to` - Possible computed jump destinations

#### DynamicJumpi
Block ends with conditional jump to computed destination
- `true_to` -  Possible computed jump destinations if true
- `false_to` - Destination if condition is false (fall-through)


### DynamicJump

```python
class DynamicJump():
    path: List[int]
    to: Optional[int]
```

Represents a dynamic jump destination in the control flow.

**Attributes**:

- `path` - Path of basic blocks leading to this jump
- `to` - Target basic block offset if known, None otherwise


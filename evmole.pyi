from typing import List, Literal, Optional, Tuple, Union

class Function:
    """
    Represents a public smart contract function.

    Attributes:
        selector (str): Function selector as a 4-byte hex string without '0x' prefix (e.g., 'aabbccdd').
        bytecode_offset (int): Starting byte offset within the EVM bytecode for the function body.
        arguments (Optional[str]): Function argument types in canonical format (e.g., 'uint256,address[]').
            None if arguments were not extracted
        state_mutability (Optional[str]): Function's state mutability ('pure', 'view', 'payable', or 'nonpayable').
            None if state mutability was not extracted
    """

    selector: str
    bytecode_offset: int
    arguments: Optional[str]
    state_mutability: Optional[str]

class StorageRecord:
    """
    Represents a storage variable record in a smart contract's storage layout.

    Attributes:
        slot (str): Storage slot number as a hex string (e.g., '0', '1b').
        offset (int): Byte offset within the storage slot (0-31).
        type (str): Variable type (e.g., 'uint256', 'mapping(address => uint256)', 'bytes32').
        reads (List[str]): List of function selectors that read from this storage location.
        writes (List[str]): List of function selectors that write to this storage location.
    """

    slot: str
    offset: int
    type: str
    reads: List[str]
    writes: List[str]

class DynamicJump:
    """
    Represents a dynamic jump destination in the control flow.

    Attributes:
        path (List[int]): Path of block IDs leading to this jump.
        to (Optional[int]): Destination block ID if known, None otherwise; use Block.start to get the bytecode offset.
    """
    path: List[int]
    to: Optional[int]

class BlockType:
    """
    Represents the type of a basic block and its control flow.
    This is an enum-like class, all child classes are derived from BlockType class
    """
    class Terminate:
        """Block terminates execution"""
        success: bool  # True for normal termination (STOP/RETURN), False for REVERT/INVALID

    class Jump:
        """Block ends with unconditional jump"""
        to: int  # Destination block ID; use Block.start to get the bytecode offset

    class Jumpi:
        """Block ends with conditional jump"""
        true_to: int   # Destination block ID if condition is true; use Block.start to get the bytecode offset
        false_to: int  # Destination block ID if condition is false; use Block.start to get the bytecode offset

    class DynamicJump:
        """Block ends with jump to computed destination"""
        to: List[DynamicJump]  # Possible computed jump destinations

    class DynamicJumpi:
        """Block ends with conditional jump to computed destination"""
        true_to: List[DynamicJump]  # Possible computed jump destinations if true
        false_to: int               # Destination block ID if condition is false; use Block.start to get the bytecode offset

class Block:
    """
    Represents a basic block in the control flow graph.

    Attributes:
        id (int): Unique block identifier (CFG key)
        start (int): Byte offset where the block's first opcode begins
        end (int): Byte offset where the block's last opcode begins
        btype (BlockType): Type of the block and its control flow.
    """
    id: int
    start: int
    end: int
    btype: BlockType

class ControlFlowGraph:
    """
    Represents the control flow graph of the contract bytecode.

    Attributes:
        blocks (List[Block]): List of basic blocks in the control flow graph.
    """
    blocks: List[Block]

class Contract:
    """
    Contains analyzed information about a smart contract.

    Attributes:
        functions (Optional[List[Function]]): List of detected contract functions.
            None if no functions were extracted
        storage (Optional[List[StorageRecord]]): List of contract storage records.
            None if storage layout was not extracted
        disassembled (Optional[List[Tuple[int, str]]]): List of bytecode instructions, where each element is [offset, instruction].
            None if disassembly was not requested
        basic_blocks (Optional[List[Tuple[int, int]]]): List of basic block ranges as (first_op, last_op) offsets.
            None if basic blocks were not requested
        control_flow_graph (Optional[ControlFlowGraph]): Control flow graph of the contract.
            None if control flow analysis was not requested
        metadata (Optional[CborMetadata]): Terminal CBOR metadata.
            None if extraction was not requested or no valid trailer exists
    """

    functions: Optional[List[Function]]
    storage: Optional[List[StorageRecord]]
    disassembled: Optional[List[Tuple[int, str]]]
    basic_blocks: Optional[List[Tuple[int, int]]]
    control_flow_graph: Optional[ControlFlowGraph]
    metadata: Optional[CborMetadata]

class CborValue:
    """
    Represents a decoded CBOR scalar or an unsupported value kept in encoded form.

    Attributes:
        type: Value kind: string, integer, bytes, bool, or undecoded.
        value: Decoded scalar. Bytes and undecoded values are returned as bytes.
    """

    type: Literal["string", "integer", "bytes", "bool", "undecoded"]
    value: Union[str, int, bytes, bool]

class CborEntry:
    """
    Represents a CBOR map entry having a text-string key.

    Attributes:
        key: Text-string map key.
        value: Decoded or preserved CBOR value.
    """

    key: str
    value: CborValue

class CborMetadata:
    """
    Represents terminal length-suffixed CBOR metadata.

    Attributes:
        bytecode_offset: Absolute byte offset of the CBOR payload.
        cbor_length: CBOR payload length, excluding the two-byte suffix.
        entries: Entries having text-string keys; other keys are skipped.
    """

    bytecode_offset: int
    cbor_length: int
    entries: List[CborEntry]

def contract_info(
    code: Union[bytes, str],
    *,
    selectors: bool = False,
    arguments: bool = False,
    state_mutability: bool = False,
    storage: bool = False,
    disassemble: bool = False,
    basic_blocks: bool = False,
    control_flow_graph: bool = False,
    metadata: bool = False,
) -> Contract:
    """
    Extracts information about a smart contract from its EVM bytecode.

    Args:
        code (Union[bytes, str]): Runtime bytecode as a hex string (with or without '0x' prefix)
            or raw bytes.
        selectors (bool, optional): When True, extracts function selectors. Defaults to False.
        arguments (bool, optional): When True, extracts function arguments. Defaults to False.
        state_mutability (bool, optional): When True, extracts function state mutability.
            Defaults to False.
        storage (bool, optional): When True, extracts the contract's storage layout.
            Defaults to False.
        disassemble (bool, optional): When True, includes disassembled bytecode.
            Defaults to False.
        basic_blocks (bool, optional): When True, extracts basic block ranges.
            Defaults to False.
        control_flow_graph (bool, optional): When True, builds control flow graph.
            Defaults to False.
        metadata (bool, optional): When True, extracts terminal CBOR metadata.
            Defaults to False.

    Returns:
        Contract: Object containing the requested smart contract information. Fields that
            weren't requested to be extracted will be None.
    """
    ...

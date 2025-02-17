from typing import List, Optional, Tuple, Union

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
        path (List[int]): Path of basic blocks leading to this jump.
        to (Optional[int]): Target basic block offset if known, None otherwise.
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
        to: int  # Destination basic block offset

    class Jumpi:
        """Block ends with conditional jump"""
        true_to: int   # Destination if condition is true
        false_to: int  # Destination if condition is false (fall-through)

    class DynamicJump:
        """Block ends with jump to computed destination"""
        to: List[DynamicJump]  # Possible computed jump destinations

    class DynamicJumpi:
        """Block ends with conditional jump to computed destination"""
        true_to: List[DynamicJump]  # Possible computed jump destinations if true
        false_to: int               # Destination if condition is false (fall-through)

class Block:
    """
    Represents a basic block in the control flow graph.

    Attributes:
        start (int): Byte offset where the block's first opcode begins
        end (int): Byte offset where the block's last opcode begins
        btype (BlockType): Type of the block and its control flow.
    """
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
    """

    functions: Optional[List[Function]]
    storage: Optional[List[StorageRecord]]
    disassembled: Optional[List[Tuple[int, str]]]
    basic_blocks: Optional[List[Tuple[int, int]]]
    control_flow_graph: Optional[ControlFlowGraph]

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

    Returns:
        Contract: Object containing the requested smart contract information. Fields that
            weren't requested to be extracted will be None.
    """
    ...

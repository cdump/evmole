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
    """

    functions: Optional[List[Function]]
    storage: Optional[List[StorageRecord]]
    disassembled: Optional[List[Tuple[int, str]]]

def contract_info(
    code: Union[bytes, str],
    *,
    selectors: bool = False,
    arguments: bool = False,
    state_mutability: bool = False,
    storage: bool = False,
    disassemble: bool = False,
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

    Returns:
        Contract: Object containing the requested smart contract information. Fields that
            weren't requested to be extracted will be None.
    """
    ...

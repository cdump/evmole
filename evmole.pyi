from typing import List, Union

def function_selectors(code: Union[bytes, str], gas_limit: int = 500000) -> List[str]:
    """
    Extracts function selectors from the given bytecode.

    Args:
        code (Union[bytes, str]): Runtime bytecode as a hex string or bytes.
        gas_limit (int, optional): Maximum gas to use. Defaults to 500000.

    Returns:
        List[str]: List of selectors encoded as hex strings.
    """
    ...

def function_arguments(code: Union[bytes, str], selector: Union[bytes, str], gas_limit: int = 50000) -> str:
    """
    Extracts function arguments for a given selector from the bytecode.

    Args:
        code (Union[bytes, str]): Runtime bytecode as a hex string or bytes.
        selector (Union[bytes, str]): Function selector as a hex string or bytes.
        gas_limit (int, optional): Maximum gas to use. Defaults to 50000.

    Returns:
        str: Arguments of the function.
    """
    ...

def function_state_mutability(code: Union[bytes, str], selector: Union[bytes, str], gas_limit: int = 500000) -> str:
    """
    Extracts function state mutability for a given selector from the bytecode.

    Args:
        code (Union[bytes, str]): Runtime bytecode as a hex string or bytes.
        selector (Union[bytes, str]): Function selector as a hex string or bytes.
        gas_limit (int, optional): Maximum gas to use. Defaults to 500000.

    Returns:
        str: "payable" | "nonpayable" | "view" | "pure"
    """
    ...

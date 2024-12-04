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
                  storage: bool = False) -> Contract
```

Extracts information about a smart contract from its EVM bytecode.

**Arguments**:

- `code` - Runtime bytecode as a hex string (with or without '0x' prefix)
  or raw bytes.
- `selectors` - When True, extracts function selectors.
- `arguments` - When True, extracts function arguments.
- `state_mutability` - When True, extracts function state mutability.
- `storage` - When True, extracts the contract's storage layout.
  

**Returns**:

- `Contract` - Object containing the requested smart contract information. Fields that
  weren't requested to be extracted will be None.

### Contract

```python
class Contract():
    functions: Optional[List[Function]]
    storage: Optional[List[StorageRecord]]
```

Contains analyzed information about a smart contract.

**Attributes**:

- `functions` - List of detected contract functions. None if no functions were extracted
- `storage` - List of contract storage records. None if storage layout was not extracted

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
  

## Deprecated API functions
Please use `contract_info()`

### function\_selectors

```python
@deprecated("Use contract_info() with selectors=True instead")
def function_selectors(code: Union[bytes, str],
                       gas_limit: int = 500000) -> List[str]
```

Extracts function selectors from the given bytecode.

**Arguments**:

- `code` _Union[bytes, str]_ - Runtime bytecode as a hex string or bytes.
- `gas_limit` _int, optional_ - Maximum gas to use. Defaults to 500000.
  

**Returns**:

- `List[str]` - List of selectors encoded as hex strings.

### function\_arguments

```python
@deprecated("Use contract_info() with arguments=True instead")
def function_arguments(code: Union[bytes, str],
                       selector: Union[bytes, str],
                       gas_limit: int = 50000) -> str
```

Extracts function arguments for a given selector from the bytecode.

**Arguments**:

- `code` _Union[bytes, str]_ - Runtime bytecode as a hex string or bytes.
- `selector` _Union[bytes, str]_ - Function selector as a hex string or bytes.
- `gas_limit` _int, optional_ - Maximum gas to use. Defaults to 50000.

**Returns**:

- `str` - Arguments of the function.

### function\_state\_mutability

```python
@deprecated("Use contract_info() with state_mutability=True instead")
def function_state_mutability(code: Union[bytes, str],
                              selector: Union[bytes, str],
                              gas_limit: int = 500000) -> str
```

Extracts function state mutability for a given selector from the bytecode.

**Arguments**:

- `code` _Union[bytes, str]_ - Runtime bytecode as a hex string or bytes.
- `selector` _Union[bytes, str]_ - Function selector as a hex string or bytes.
- `gas_limit` _int, optional_ - Maximum gas to use. Defaults to 500000.

**Returns**:

- `str` - "payable" | "nonpayable" | "view" | "pure"

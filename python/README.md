# EVMole Python

EVMole Python is built with [PyO3](https://pyo3.rs/) for different operating systems and Python versions. In most cases, pip will install a pre-built wheel. However, the source distribution (sdist) is also published, allowing installation on other architectures and Python versions (this is automatically tested).

## Installation
To install or upgrade EVMole, use pip:
```bash
$ pip install evmole --upgrade
```

<!-- manually generated with pydoc-markdown, rename 'evmole.pyi' to 'evmole.py' -->
## API

### function\_selectors

```python
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

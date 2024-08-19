# EVMole Python

EVMole Python is built with [PyO3](https://pyo3.rs/) for different operating systems and Python versions. In most cases, pip will install a pre-built wheel. However, the source distribution (sdist) is also published, allowing installation on other architectures and Python versions (this is automatically tested).

## Installation
To install or upgrade EVMole, use pip:
```bash
$ pip install evmole --upgrade
```

## Usage
Here's a basic example of how to use EVMole in Python:
```python
from evmole import function_arguments, function_selectors

code = '0x6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256'
print( function_selectors(code) )
# Output(list): ['2125b65b', 'b69ef8a8']

print( function_arguments(code, '2125b65b') )
# Output(str): 'uint32,address,uint224'
```

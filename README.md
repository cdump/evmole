# EVMole

[![try it online](https://img.shields.io/badge/Try_It_Online-github.io-brightgreen)](https://cdump.github.io/evmole/)
[![npm](https://img.shields.io/npm/v/evmole)](https://www.npmjs.com/package/evmole)
[![Crates.io](https://img.shields.io/crates/v/evmole?color=e9b44f)](https://crates.io/crates/evmole)
[![PyPI](https://img.shields.io/pypi/v/evmole?color=006dad)](https://pypi.org/project/evmole)

EVMole is a powerful library that extracts information from Ethereum Virtual Machine (EVM) bytecode, including [function selectors](https://docs.soliditylang.org/en/latest/abi-spec.html#function-selector), arguments, and [state mutability](https://docs.soliditylang.org/en/latest/contracts.html#state-mutability), even for unverified contracts.


## Key Features

- Multi-language support: Available as [JavaScript](#javascript), [Rust](#rust), and [Python](#python) libraries.
- High accurancy and performance: [Outperforms](#benchmark) existing tools.
- Broad compatibility: Tested with both Solidity and Vyper compiled contracts.
- Lightweight: Clean codebase with minimal external dependencies.
- Unverified contract analysis: Extracts information even from unverified bytecode.


## Usage
### JavaScript
[API documentation](./javascript/#api) and [usage examples](./javascript#usage) (node, vite, webpack, parcel, esbuild)
```sh
$ npm i evmole
```
```javascript
import { functionSelectors, functionArguments, functionStateMutability } from 'evmole'

const code = '0x6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256'

console.log(functionSelectors(code));                   // [ '2125b65b', 'b69ef8a8' ]
console.log(functionArguments(code, '2125b65b'));       // 'uint32,address,uint224'
console.log(functionStateMutability(code, '2125b65b')); // 'pure'
```

### Rust
Documentation is available on [docs.rs](https://docs.rs/evmole/latest/evmole/)
```rust
let code = hex::decode("6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256").unwrap();

println!("{:x?} | {} | {:?}",
    evmole::function_selectors(&code, 0),
    evmole::function_arguments(&code, &[0x21, 0x25, 0xb6, 0x5b], 0),
    evmole::function_state_mutability(&code, &[0x21, 0x25, 0xb6, 0x5b], 0),
);
// [[21, 25, b6, 5b], [b6, 9e, f8, a8]] | uint32,address,uint224 | Pure
```

### Python
[API documentation](./python/#api)
```sh
$ pip install evmole --upgrade
```
```python
from evmole import function_selectors, function_arguments, function_state_mutability

code = '0x6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256'

print(function_selectors(code))                    # ['2125b65b', 'b69ef8a8']
print(function_arguments(code, '2125b65b'))        # uint32,address,uint224
print(function_state_mutability(code, '2125b65b')) # pure
```

### Foundry
<a href="https://getfoundry.sh/">Foundy's cast</a> uses the Rust implementation of EVMole
```sh

$ cast selectors $(cast code 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2)
0x06fdde03	
0x095ea7b3	address,uint256
0x18160ddd	
0x23b872dd	address,address,uint256
...

$ cast selectors --resolve $(cast code 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2)
0x06fdde03	                       	name()
0x095ea7b3	address,uint256        	approve(address,uint256)
0x18160ddd	                       	totalSupply()
0x23b872dd	address,address,uint256	transferFrom(address,address,uint256)
...
```

## Benchmark

### function selectors
<i>FP/FN</i> - [False Positive/False Negative](https://en.wikipedia.org/wiki/False_positives_and_false_negatives) errors; <b>smaller is better</b>

<table>
 <tr>
  <td>Dataset</td>
  <td></td>
  <td><b><i>evmole</i><b> <a href="benchmark/providers/evmole-rs/"><b><i>rs</i></b></a> · <a href="benchmark/providers/evmole-js/"><b><i>js</i></b></a> · <a href="benchmark/providers/evmole-py/"><b><i>py</i></b></a></td>
  <td><a href="benchmark/providers/whatsabi/"><b><i>whatsabi</i></b></a></td>
  <td><a href="benchmark/providers/sevm/"><b><i>sevm</i></b></a></td>
  <td><a href="benchmark/providers/evm-hound-rs/"><b><i>evmhound</i></b></a></td>
  <td><a href="benchmark/providers/heimdall-rs/"><b><i>heimdall</i></b></a></td>
  <td><a href="benchmark/providers/simple/"><b><i>smpl</i></b></a></td>
 </tr>
 <tr>
  <td rowspan="5"><b>largest1k</b><br><sub>1000<br>addresses<br><br>24427<br>functions</sub></td>
  <td><i>FP <sub>addrs</sub></i></td>
  <td>1 🥈</td>
  <td>0 🥇</td>
  <td>0 🥇</td>
  <td>75</td>
  <td>18</td>
  <td>95</td>
 </tr>
 <tr>
  <td><i>FN <sub>addrs</sub></i></td>
  <td>0 🥇</td>
  <td>0 🥇</td>
  <td>0 🥇</td>
  <td>40</td>
  <td>111</td>
  <td>9</td>
 </tr>
 <tr>
  <td><i>FP <sub>funcs</sub></i></td>
  <td>192 🥈</td>
  <td>0 🥇</td>
  <td>0 🥇</td>
  <td>720</td>
  <td>600</td>
  <td>749</td>
 </tr>
 <tr>
  <td><i>FN <sub>funcs</sub></i></td>
  <td>0 🥇</td>
  <td>0 🥇</td>
  <td>0 🥇</td>
  <td>191</td>
  <td>147</td>
  <td>12</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>0.4s · 0.8s · 0.6s</td>
  <td>2.9s</td>
  <td>38s<sup>(*)</sup></td>
  <td>0.5s</td>
  <td>341s<sup>(*)</sup></td>
  <td>1.8s</td>
 </tr>
 <tr><td colspan="8"></td></tr>
 <tr>
  <td rowspan="5"><b>random50k</b><br><sub>50000<br>addresses<br><br>1171102<br>functions</sub></td>
  <td><i>FP <sub>addrs</sub></i></td>
  <td>1 🥇</td>
  <td>43</td>
  <td>1</td>
  <td>693</td>
  <td>3</td>
  <td>4136</td>
 </tr>
 <tr>
  <td><i>FN <sub>addrs</sub></i></td>
  <td>9 🥇</td>
  <td>11</td>
  <td>36</td>
  <td>2903</td>
  <td>4708</td>
  <td>77</td>
 </tr>
 <tr>
  <td><i>FP <sub>funcs</sub></i></td>
  <td>3 🥇</td>
  <td>51</td>
  <td>3</td>
  <td>10798</td>
  <td>29</td>
  <td>14652</td>
 </tr>
 <tr>
  <td><i>FN <sub>funcs</sub></i></td>
  <td>10 🥇</td>
  <td>12</td>
  <td>587</td>
  <td>3538</td>
  <td>6098</td>
  <td>96</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>4.5s · 12s · 10s</td>
  <td>49s</td>
  <td>1427s<sup>(*)</sup></td>
  <td>5.8s</td>
  <td>8576s<sup>(*)</sup></td>
  <td>49s</td>
 </tr>
 <tr><td colspan="8"></td></tr>
 <tr>
  <td rowspan="5"><b>vyper</b><br><sub>780<br>addresses<br><br>21244<br>functions</sub></td>
  <td><i>FP <sub>addrs</sub></i></td>
  <td>0 🥇</td>
  <td>30</td>
  <td>0</td>
  <td>19</td>
  <td>0</td>
  <td>185</td>
 </tr>
 <tr>
  <td><i>FN <sub>addrs</sub></i></td>
  <td>0 🥇</td>
  <td>780</td>
  <td>21</td>
  <td>300</td>
  <td>780</td>
  <td>480</td>
 </tr>
 <tr>
  <td><i>FP <sub>funcs</sub></i></td>
  <td>0 🥇</td>
  <td>30</td>
  <td>0</td>
  <td>19</td>
  <td>0</td>
  <td>197</td>
 </tr>
 <tr>
  <td><i>FN <sub>funcs</sub></i></td>
  <td>0 🥇</td>
  <td>21244</td>
  <td>336</td>
  <td>8273</td>
  <td>21244</td>
  <td>12971</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>0.4s · 0.7s · 0.5s</td>
  <td>2.2s</td>
  <td>60s<sup>(*)</sup></td>
  <td>0.4s</td>
  <td>27s<sup>(*)</sup></td>
  <td>1.1s</td>
 </tr>
</table>

### function arguments
<i>Errors</i> - when at least 1 argument is incorrect: `(uint256,string)` ≠ `(uint256,bytes)`

<table>
 <tr>
  <td>Dataset</td>
  <td></td>
  <td><b><i>evmole</i><b> <a href="benchmark/providers/evmole-rs/"><b><i>rs</i></b></a> · <a href="benchmark/providers/evmole-js/"><b><i>js</i></b></a> · <a href="benchmark/providers/evmole-py/"><b><i>py</i></b></a></td>
  <td><a href="benchmark/providers/heimdall-rs/"><b><i>heimdall</i></b></a></td>
  <td><a href="benchmark/providers/simple/"><b><i>smpl</i></b></a></td>
 </tr>
 <tr>
  <td rowspan="2"><b>largest1k</b><br><sub>24427<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>14.0% 🥇<br><sub>3417</sub></td>
  <td>31.1%<br><sub>7593</sub></td>
  <td>58.3%<br><sub>14242</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>1.0s · 8.3s · 3.5s</td>
  <td>342s<sup>(*)</sup></td>
  <td>0.7s</td>
 </tr>
 <tr><td colspan="5"></td></tr>
 <tr>
  <td rowspan="2"><b>random50k</b><br><sub>1171102<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>4.5% 🥇<br><sub>52777</sub></td>
  <td>19.4%<br><sub>227612</sub></td>
  <td>54.9%<br><sub>643213</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>23s · 263s · 104s</td>
  <td>8544s<sup>(*)</sup></td>
  <td>9.7s</td>
 </tr>
 <tr><td colspan="5"></td></tr>
 <tr>
  <td rowspan="2"><b>vyper</b><br><sub>21244<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>49.6% 🥇<br><sub>10544</sub></td>
  <td>100.0%<br><sub>21244</sub></td>
  <td>56.8%<br><sub>12077</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>0.7s · 5.2s · 2.2s</td>
  <td>28s<sup>(*)</sup></td>
  <td>0.5s</td>
 </tr>
</table>

### function state mutability

<i>Errors</i> - Results are not equal (treating `view` and `pure` as equivalent to `nonpayable`)

<i>Errors strict</i> - Results are strictly unequal (`nonpayable` ≠ `view`). Some ABIs mark `pure`/`view` functions as `nonpayable`, so not all strict errors indicate real issues.

<table>
 <tr>
  <td>Dataset</td>
  <td></td>
  <td><b><i>evmole</i><b> <a href="benchmark/providers/evmole-rs/"><b><i>rs</i></b></a> · <a href="benchmark/providers/evmole-js/"><b><i>js</i></b></a> · <a href="benchmark/providers/evmole-py/"><b><i>py</i></b></a></td>
  <td><a href="benchmark/providers/whatsabi/"><b><i>whatsabi</i></b></a></td>
  <td><a href="benchmark/providers/sevm/"><b><i>sevm</i></b></a></td>
  <td><a href="benchmark/providers/heimdall-rs/"><b><i>heimdall</i></b></a></td>
  <td><a href="benchmark/providers/simple/"><b><i>smpl</i></b></a></td>
 </tr>
 <tr>
  <td rowspan="3"><b>largest1k</b><br><sub>24427<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>0.0% 🥇<br><sub>0</sub></td>
  <td>68.1%<br><sub>16623</sub></td>
  <td>2.1%<br><sub>501</sub></td>
  <td>25.4%<br><sub>6201</sub></td>
  <td>2.6%<br><sub>643</sub></td>
 </tr>
 <tr>
  <td><i>Errors strict</i></td>
  <td>19.3% 🥇<br><sub>4718</sub></td>
  <td>79.3%<br><sub>19370</sub></td>
  <td>59.0%<br><sub>14417</sub></td>
  <td>54.9%<br><sub>13403</sub></td>
  <td>60.9%<br><sub>14864</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>7.9s · 17s · 10s</td>
  <td>3.7s</td>
  <td>37s<sup>(*)</sup></td>
  <td>339s<sup>(*)</sup></td>
  <td>0.7s</td>
 </tr>
 <tr><td colspan="6"></td></tr>
 <tr>
  <td rowspan="3"><b>random50k</b><br><sub>1160861<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>0.0% 🥇<br><sub>35</sub></td>
  <td>30.2%<br><sub>351060</sub></td>
  <td>0.3%<br><sub>3887</sub></td>
  <td>11.6%<br><sub>134195</sub></td>
  <td>2.2%<br><sub>24961</sub></td>
 </tr>
 <tr>
  <td><i>Errors strict</i></td>
  <td>6.8% 🥇<br><sub>78676</sub></td>
  <td>58.1%<br><sub>674922</sub></td>
  <td>55.7%<br><sub>647070</sub></td>
  <td>27.7%<br><sub>321494</sub></td>
  <td>57.7%<br><sub>670318</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>226s · 523s · 309s</td>
  <td>80s</td>
  <td>1709s<sup>(*)</sup></td>
  <td>8151s<sup>(*)</sup></td>
  <td>9.4s</td>
 </tr>
 <tr><td colspan="6"></td></tr>
 <tr>
  <td rowspan="3"><b>vyper</b><br><sub>21166<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>0.5% 🥇<br><sub>110</sub></td>
  <td>100.0%<br><sub>21166</sub></td>
  <td>77.8%<br><sub>16462</sub></td>
  <td>100.0%<br><sub>21166</sub></td>
  <td>1.8%<br><sub>390</sub></td>
 </tr>
 <tr>
  <td><i>Errors strict</i></td>
  <td>11.4% 🥇<br><sub>2410</sub></td>
  <td>100.0%<br><sub>21166</sub></td>
  <td>91.0%<br><sub>19253</sub></td>
  <td>100.0%<br><sub>21166</sub></td>
  <td>59.6%<br><sub>12610</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>3.7s · 8.7s · 5.1s</td>
  <td>2.2s</td>
  <td>59s<sup>(*)</sup></td>
  <td>28s<sup>(*)</sup></td>
  <td>0.6s</td>
 </tr>
</table>

See [benchmark/README.md](./benchmark/) for the methodology and commands to reproduce these results

<i>versions: evmole v0.4.1; <a href="https://github.com/shazow/whatsabi">whatsabi</a> v0.14.1; <a href="https://github.com/acuarica/evm">sevm</a> v0.6.19; <a href="https://github.com/g00dv1n/evm-hound-rs">evm-hound-rs</a> v0.1.4; <a href="https://github.com/Jon-Becker/heimdall-rs">heimdall-rs</a> v0.8.4</i>

<sup>(*)</sup>: <b>sevm</b> and <b>heimdall-rs</b> are full decompilers, not limited to extracting function selectors

## How it works

Short: Executes code with a custom EVM and traces CALLDATA usage.

Long: TODO

## License
MIT

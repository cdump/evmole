# EVMole

[![npm](https://img.shields.io/npm/v/evmole)](https://www.npmjs.com/package/evmole)
[![Crates.io](https://img.shields.io/crates/v/evmole?color=e9b44f)](https://crates.io/crates/evmole)
[![PyPI](https://img.shields.io/pypi/v/evmole?color=006dad)](https://pypi.org/project/evmole)
[![license](https://img.shields.io/github/license/cdump/evmole)](./LICENSE)

This library extracts [function selectors](https://docs.soliditylang.org/en/latest/abi-spec.html#function-selector) and arguments from Ethereum Virtual Machine (EVM) bytecode, even for unverified contracts.

- JavaScript, Rust and Python implementations
- Clean code with zero external dependencies (py & js)
- [Faster and more accurate](#Benchmark) than other existing tools
- Tested on Solidity and Vyper compiled contracts

[Try it online](https://cdump.github.io/evmole/)

## Usage

### JavaScript
```sh
$ npm i evmole
```
```javascript
import {functionArguments, functionSelectors} from 'evmole'
// Also supported: const e = require('evmole'); e.functionSelectors();

const code = '0x6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256'
console.log( functionSelectors(code) )
// Output(list): [ '2125b65b', 'b69ef8a8' ]

console.log( functionArguments(code, '2125b65b') )
// Output(str): 'uint32,address,uint224'
```

### Rust
Documentation available on [docs.rs](https://docs.rs/evmole/latest/evmole/)
```rust
let code = hex::decode("6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256").unwrap();

println!("{:x?}", evmole::function_selectors(&code, 0));
// Output(Vec<[u8;4]>): [[21, 25, b6, 5b], [b6, 9e, f8, a8]]

println!("{}", evmole::function_arguments(&code, &[0x21, 0x25, 0xb6, 0x5b], 0));
// Output(String): uint32,address,uint224
```

### Python
```sh
$ pip install evmole --upgrade
```
```python
from evmole import function_arguments, function_selectors

code = '0x6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256'
print( function_selectors(code) )
# Output(list): ['2125b65b', 'b69ef8a8']

print( function_arguments(code, '2125b65b') )
# Output(str): 'uint32,address,uint224'
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

See [examples](./examples) for more

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
  <td rowspan="5"><b>largest1k</b><br><sub>1000<br>contracts<br><br>24427<br>functions</sub></td>
  <td><i>FP <sub>contracts</sub></i></td>
  <td>1 🥈</td>
  <td>0 🥇</td>
  <td>0 🥇</td>
  <td>75</td>
  <td>68</td>
  <td>95</td>
 </tr>
 <tr>
  <td><i>FN <sub>contracts</sub></i></td>
  <td>0 🥇</td>
  <td>0 🥇</td>
  <td>0 🥇</td>
  <td>40</td>
  <td>103</td>
  <td>9</td>
 </tr>
 <tr>
  <td><i>FP <sub>functions</sub></i></td>
  <td>192 🥈</td>
  <td>0 🥇</td>
  <td>0 🥇</td>
  <td>720</td>
  <td>650</td>
  <td>749</td>
 </tr>
 <tr>
  <td><i>FN <sub>functions</sub></i></td>
  <td>0 🥇</td>
  <td>0 🥇</td>
  <td>0 🥇</td>
  <td>191</td>
  <td>114</td>
  <td>12</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>0.6s · 1.6s · 2.1s</td>
  <td>2.9s</td>
  <td>34.8s<sup>(*)</sup></td>
  <td>0.7s</td>
  <td>379.5s<sup>(*)</sup></td>
  <td>1.7s</td>
 </tr>
 <tr><td colspan="8"></td></tr>
 <tr>
  <td rowspan="5"><b>random50k</b><br><sub>50000<br>contracts<br><br>1171102<br>functions</sub></td>
  <td><i>FP <sub>contracts</sub></i></td>
  <td>1 🥇</td>
  <td>43</td>
  <td>1</td>
  <td>693</td>
  <td>519</td>
  <td>4136</td>
 </tr>
 <tr>
  <td><i>FN <sub>contracts</sub></i></td>
  <td>9 🥇</td>
  <td>11</td>
  <td>85</td>
  <td>2903</td>
  <td>4669</td>
  <td>77</td>
 </tr>
 <tr>
  <td><i>FP <sub>functions</sub></i></td>
  <td>3 🥇</td>
  <td>51</td>
  <td>3</td>
  <td>10798</td>
  <td>545</td>
  <td>14652</td>
 </tr>
 <tr>
  <td><i>FN <sub>functions</sub></i></td>
  <td>10 🥇</td>
  <td>12</td>
  <td>1759</td>
  <td>3538</td>
  <td>4943</td>
  <td>96</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>9.8s · 20.2s · 39s</td>
  <td>55.9s</td>
  <td>1343s<sup>(*)</sup></td>
  <td>11.5s</td>
  <td>9881.1s<sup>(*)</sup></td>
  <td>47.9s</td>
 </tr>
 <tr><td colspan="8"></td></tr>
 <tr>
  <td rowspan="5"><b>vyper</b><br><sub>780<br>contracts<br><br>21244<br>functions</sub></td>
  <td><i>FP <sub>contracts</sub></i></td>
  <td>0 🥇</td>
  <td>30</td>
  <td>0</td>
  <td>19</td>
  <td>780</td>
  <td>185</td>
 </tr>
 <tr>
  <td><i>FN <sub>contracts</sub></i></td>
  <td>0 🥇</td>
  <td>780</td>
  <td>21</td>
  <td>300</td>
  <td>780</td>
  <td>480</td>
 </tr>
 <tr>
  <td><i>FP <sub>functions</sub></i></td>
  <td>0 🥇</td>
  <td>30</td>
  <td>0</td>
  <td>19</td>
  <td>780</td>
  <td>197</td>
 </tr>
 <tr>
  <td><i>FN <sub>functions</sub></i></td>
  <td>0 🥇</td>
  <td>21244</td>
  <td>336</td>
  <td>8273</td>
  <td>21244</td>
  <td>12971</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>0.4s · 0.8s · 1.2s</td>
  <td>2.1s</td>
  <td>55.1s<sup>(*)</sup></td>
  <td>0.4s</td>
  <td>29.1s<sup>(*)</sup></td>
  <td>1.1s</td>
 </tr>
</table>

### function arguments
<i>Errors</i> - when at least 1 argument is incorrect: `(uint256,string)` != `(uint256,bytes)`; <b>smaller is better</b>

<table>
 <tr>
  <td>Dataset</td>
  <td></td>
  <td><b><i>evmole</i><b> <a href="benchmark/providers/evmole-rs/"><b><i>rs</i></b></a> · <a href="benchmark/providers/evmole-js/"><b><i>js</i></b></a> · <a href="benchmark/providers/evmole-py/"><b><i>py</i></b></a></td>
  <td><a href="benchmark/providers/heimdall-rs/"><b><i>heimdall-rs</i></b></a></td>
  <td><a href="benchmark/providers/simple/"><b><i>simple</i></b></a></td>
 </tr>
 <tr>
  <td rowspan="2"><b>largest1k</b><br><sub>1000<br>contracts<br><br>24427<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>14.0%, 3417 🥇</td>
  <td>31.1%, 7603</td>
  <td>58.3%, 14242</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>1.2s · 12.6s · 30.1s</td>
  <td>385.6s<sup>(*)</sup></td>
  <td>0.6s</td>
 </tr>
 <tr><td colspan="5"></td></tr>
 <tr>
  <td rowspan="2"><b>random50k</b><br><sub>50000<br>contracts<br><br>1171102<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>4.5%, 52777 🥇</td>
  <td>19.4%, 227078</td>
  <td>54.9%, 643213</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>33.7s · 337.7s · 1002.4s</td>
  <td>9594.6s<sup>(*)</sup></td>
  <td>9.6s</td>
 </tr>
 <tr><td colspan="5"></td></tr>
 <tr>
  <td rowspan="2"><b>vyper</b><br><sub>780<br>contracts<br><br>21244<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>49.6%, 10544 🥇</td>
  <td>100.0%, 21244</td>
  <td>56.8%, 12077</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>0.8s · 8.0s · 16.9s</td>
  <td>29.6s<sup>(*)</sup></td>
  <td>0.5s</td>
 </tr>
</table>

See [benchmark/README.md](./benchmark/) for the methodology and commands to reproduce these results

<i>versions: evmole v0.3.7; <a href="https://github.com/shazow/whatsabi">whatsabi</a> v0.12.0; <a href="https://github.com/acuarica/evm">sevm</a> v0.6.18; <a href="https://github.com/g00dv1n/evm-hound-rs">evm-hound-rs</a> v0.1.4; <a href="https://github.com/Jon-Becker/heimdall-rs">heimdall-rs</a> v0.8.3</i>

<sup>(*)</sup>: <b>sevm</b> and <b>heimdall-rs</b> are full decompilers, not limited to extracting function selectors

## How it works

Short: Executes code with a custom EVM and traces CALLDATA usage.

Long: TODO

## License
MIT

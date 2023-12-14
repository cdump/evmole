# EVMole

[![npm](https://img.shields.io/npm/v/evmole)](https://www.npmjs.com/package/evmole)
[![PyPI](https://img.shields.io/pypi/v/evmole?color=006dad)](https://pypi.org/project/evmole)
[![license](https://img.shields.io/github/license/cdump/evmole)](./LICENSE)

Extracts [function selectors](https://docs.soliditylang.org/en/latest/abi-spec.html#function-selector) and arguments from EVM bytecode, even for unverified contracts.

- Python & JavaScript implementations
- Clean code with zero dependencies
- [Faster and more accurate](#Benchmark) than other tools
- Tested on Solidity and Vyper compiled contracts

[Try it online](https://cdump.github.io/evmole/)

## Usage

### JavaScript
```sh
$ npm i evmole
```
```javascript
import {functionSelectors} from 'evmole'
// Also supported: const e = require('evmole'); e.functionSelectors();

const code = '0x6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256fea2646970667358221220fbd308f142157eaf0fdc0374a3f95f796b293d35c337d2d9665b76dfc69501ea64736f6c63430008170033'
console.log( functionSelectors(code) )
// Output(list): [ '2125b65b', 'b69ef8a8' ]

console.log( functionArguments(code, '2125b65b') )
// Output(str): 'uint32,address,uint224'
```

### Python
```sh
$ pip install evmole --upgrade
```
```python
from evmole import function_arguments, function_selectors

code = '0x6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256fea2646970667358221220fbd308f142157eaf0fdc0374a3f95f796b293d35c337d2d9665b76dfc69501ea64736f6c63430008170033'
print( function_selectors(code) )
# Output(list): ['2125b65b', 'b69ef8a8']

print( function_arguments(code, '2125b65b') )
# Output(str): 'uint32,address,uint224'
```

See [examples](./examples) for more

## Benchmark

### function selectors
<i>FP/FN</i> - [False Positive/False Negative](https://en.wikipedia.org/wiki/False_positives_and_false_negatives) errors; <b>smaller is better</b>

<table>
 <tr>
  <td>Dataset</td>
  <td></td>
  <td><a href="benchmark/providers/evmole-js/"><b><i>evmole-js</i></b></a> (<a href="benchmark/providers/evmole-py/"><b><i>py</i></b></a>)</td>
  <td><a href="benchmark/providers/whatsabi/"><b><i>whatsabi</i></b></a></td>
  <td><a href="benchmark/providers/evm-hound-rs/"><b><i>evm-hound-rs</i></b></a></td>
  <td><a href="benchmark/providers/heimdall-rs/"><b><i>heimdall-rs</i></b></a></td>
  <td><a href="benchmark/providers/simple/"><b><i>simple</i></b></a></td>
 </tr>
 <tr>
  <td rowspan="5"><b>largest1k</b><br><sub>1000<br>contracts<br><br>24427<br>functions</sub></td>
  <td><i>FP <sub>contracts</sub></i></td>
  <td>1 :1st_place_medal:</td>
  <td>38</td>
  <td>75</td>
  <td>18</td>
  <td>95</td>
 </tr>
 <tr>
  <td><i>FN <sub>contracts</sub></i></td>
  <td>0 :1st_place_medal:</td>
  <td>8</td>
  <td>40</td>
  <td>103</td>
  <td>9</td>
 </tr>
 <tr>
  <td><i>FP <sub>functions</sub></i></td>
  <td>192 :2nd_place_medal:</td>
  <td>38 :1st_place_medal:</td>
  <td>720</td>
  <td>600</td>
  <td>749</td>
 </tr>
 <tr>
  <td><i>FN <sub>functions</sub></i></td>
  <td>0 :1st_place_medal:</td>
  <td>8 :2nd_place_medal:</td>
  <td>191</td>
  <td>116</td>
  <td>12</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>1.6s (1.8s)</td>
  <td>3.5s</td>
  <td>1.1s</td>
  <td>691.7s</td>
  <td>1.8s</td>
 </tr>
 <tr><td colspan="7"></td></tr>
 <tr>
  <td rowspan="5"><b>random50k</b><br><sub>50000<br>contracts<br><br>1171102<br>functions</sub></td>
  <td><i>FP <sub>contracts</sub></i></td>
  <td>1 :1st_place_medal:</td>
  <td>251</td>
  <td>693</td>
  <td rowspan="5">waiting fixes</td>
  <td>4136</td>
 </tr>
 <tr>
  <td><i>FN <sub>contracts</sub></i></td>
  <td>9 :1st_place_medal:</td>
  <td>31</td>
  <td>2903</td>
  <!-- -->
  <td>77</td>
 </tr>
 <tr>
  <td><i>FP <sub>functions</sub></i></td>
  <td>3 :1st_place_medal:</td>
  <td>261</td>
  <td>10798</td>
  <td>14652</td>
 </tr>
 <tr>
  <td><i>FN <sub>functions</sub></i></td>
  <td>10 :1st_place_medal:</td>
  <td>32</td>
  <td>3538</td>
  <td>96</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>19.6s (32.8s)</td>
  <td>67.1s</td>
  <td>11.9s</td>
  <td>34.4s</td>
 </tr>
 <tr><td colspan="7"></td></tr>
 <tr>
  <td rowspan="5"><b>vyper</b><br><sub>780<br>contracts<br><br>21244<br>functions</sub></td>
  <td><i>FP <sub>contracts</sub></i></td>
  <td>0 :1st_place_medal:</td>
  <td>178</td>
  <td>19</td>
  <td>0</td>
  <td>185</td>
 </tr>
 <tr>
  <td><i>FN <sub>contracts</sub></i></td>
  <td>0 :1st_place_medal:</td>
  <td>780</td>
  <td>300</td>
  <td>780</td>
  <td>480</td>
 </tr>
 <tr>
  <td><i>FP <sub>functions</sub></i></td>
  <td>0 :1st_place_medal:</td>
  <td>181</td>
  <td>19</td>
  <td>0</td>
  <td>197</td>
 </tr>
 <tr>
  <td><i>FN <sub>functions</sub></i></td>
  <td>0 :1st_place_medal:</td>
  <td>21244</td>
  <td>8273</td>
  <td>21244</td>
  <td>12971</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>1.5s (1.4s)</td>
  <td>2.4s</td>
  <td>1.0s</td>
  <td>9.9s</td>
  <td>1.3s</td>
 </tr>
</table>

### function arguments
<i>Errors</i> - when at least 1 argument is incorrect: `(uint256,string)` != `(uint256,bytes)`; <b>smaller is better</b>

<table>
 <tr>
  <td>Dataset</td>
  <td></td>
  <td><a href="benchmark/providers/evmole-js/"><b><i>evmole-js</i></b></a> (<a href="benchmark/providers/evmole-py/"><b><i>py</i></b></a>)</td>
  <td><a href="benchmark/providers/heimdall-rs/"><b><i>heimdall-rs</i></b></a></td>
  <td><a href="benchmark/providers/simple/"><b><i>simple</i></b></a></td>
 </tr>
 <tr>
  <td rowspan="2"><b>largest1k</b><br><sub>1000<br>contracts<br><br>24427<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>15.0%, 3671 :1st_place_medal:</td>
  <td>42.6%, 10407</td>
  <td>58.3%, 14242</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>7.1s (13.1s)</td>
  <td>693.4s</td>
  <td>1.2s</td>
 </tr>
 <tr><td colspan="5"></td></tr>
 <tr>
  <td rowspan="2"><b>random50k</b><br><sub>50000<br>contracts<br><br>1171102<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>5.4%, 63753 :1st_place_medal:</td>
  <td rowspan="2">waiting fixes</td>
  <td>54.9%, 643213</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>200.4s (413.3s)</td>
  <td>8.8s</td>
 </tr>
 <tr><td colspan="5"></td></tr>
 <tr>
  <td rowspan="2"><b>vyper</b><br><sub>780<br>contracts<br><br>21244<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>52.3%, 11103 :1st_place_medal:</td>
  <td>100.0%, 21244</td>
  <td>56.8%, 12077</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>8.1s (16.3s)</td>
  <td>10.2s</td>
  <td>1.0s</td>
 </tr>
</table>

See [benchmark/README.md](./benchmark/) for the methodology and commands to reproduce these results

<i>versions: evmole v0.2.1; whatsabi v0.9.1; evm-hound-rs v0.1.4; heimdall-rs v0.7.1</i>

## How it works

Short: Executes code with a custom EVM and traces CALLDATA usage.

Long: TODO

## License
MIT

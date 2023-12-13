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
<i>FP/FN</i> - [False Positive/False Negative](https://en.wikipedia.org/wiki/False_positives_and_false_negatives) errors; smaller is better

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
  <td rowspan="3"><i><b>largest1k</b><br>1000 contracts<br>24427 functions</i></td>
  <td><i>FP/FN contracts:</i></td>
  <td>1 / 0 :1st_place_medal:</td>
  <td>38 / 8</td>
  <td>75 / 40</td>
  <td>18 / 103</td>
  <td>95 / 9</td>
 </tr>
 <tr>
  <td><i>FP/FN functions:</i></td>
  <td>192 / 0 :2nd_place_medal: :1st_place_medal:</td>
  <td>38 / 8 :1st_place_medal: :2nd_place_medal:</td>
  <td>720 / 191</td>
  <td>600 / 116</td>
  <td>749 / 12</td>
 </tr>
 <tr>
  <td><i>Time:</i></td>
  <td>1.6s (1.74s)</td>
  <td>3.54s</td>
  <td>1.1s</td>
  <td>691.68s</td>
  <td>1.85s</td>
 </tr>
 <tr><td colspan="7"></td></tr>
 <tr>
  <td rowspan="3"><i><b>random50k</b><br>50000 contracts<br>1171102 functions</i></td>
  <td><i>FP/FN contracts:</i></td>
  <td>1 / 9 :1st_place_medal:</td>
  <td>251 / 31</td>
  <td>693 / 2903</td>
  <td rowspan="3">waiting fixes</td>
  <td>4136 / 77</td>
 </tr>
 <tr>
  <td><i>FP/FN functions:</i></td>
  <td>3 / 10 :1st_place_medal:</td>
  <td>261 / 32</td>
  <td>10798 / 3538</td>
  <td>14652 / 96</td>
 </tr>
 <tr>
  <td><i>Time:</i></td>
  <td>18.81s (32.27s)</td>
  <td>67.13s</td>
  <td>11.95s</td>
  <td>34.39s</td>
 </tr>
 <tr><td colspan="7"></td></tr>
 <tr>
  <td rowspan="3"><i><b>vyper</b><br>780 contracts<br>21244 functions</i></td>
  <td><i>FP/FN contracts:</i></td>
  <td>0 / 0 :1st_place_medal:</td>
  <td>178 / 780</td>
  <td>19 / 300</td>
  <td>0 / 780</td>
  <td>185 / 480</td>
 </tr>
 <tr>
  <td><i>FP/FN functions:</i></td>
  <td>0 / 0 :1st_place_medal:</td>
  <td>181 / 21244</td>
  <td>19 / 8273</td>
  <td>0 / 21244</td>
  <td>197 / 12971</td>
 </tr>
 <tr>
  <td><i>Time:</i></td>
  <td>1.42s (1.28s)</td>
  <td>2.39s</td>
  <td>0.97s</td>
  <td>9.86s</td>
  <td>1.34s</td>
 </tr>
</table>

### function arguments
<i>errors</i> - when at least 1 argument is incorrect: `(uint256,string)` != `(uint256,bytes)`; smaller is better

<table>
 <tr>
  <td>Dataset</td>
  <td></td>
  <td><a href="benchmark/providers/evmole-js/"><b><i>evmole-js</i></b></a> (<a href="benchmark/providers/evmole-py/"><b><i>py</i></b></a>)</td>
  <td><a href="benchmark/providers/heimdall-rs/"><b><i>heimdall-rs</i></b></a></td>
  <td><a href="benchmark/providers/simple/"><b><i>simple</i></b></a></td>
 </tr>
 <tr>
  <td rowspan="2"><i><b>largest1k</b><br>1000 contracts<br>24427 functions</i></td>
  <td><i>errors:</i></td>
  <td>15.1%, 3677 :1st_place_medal:</td>
  <td>42.6%, 10407</td>
  <td>58.3%, 14242</td>
 </tr>
 <tr>
  <td><i>Time:</i></td>
  <td>6.84s (13.02s)</td>
  <td>693.42s</td>
  <td>1.17s</td>
 </tr>
 <tr><td colspan="5"></td></tr>
 <tr>
  <td rowspan="2"><i><b>random50k</b><br>50000 contracts<br>1171102 functions</i></td>
  <td><i>errors:</i></td>
  <td>5.4%, 63774 :1st_place_medal:</td>
  <td rowspan="2">waiting fixes</td>
  <td>54.9%, 643213</td>
 </tr>
 <tr>
  <td><i>Time:</i></td>
  <td>185.03s (402.58s)</td>
  <td>8.76s</td>
 </tr>
 <tr><td colspan="5"></td></tr>
 <tr>
  <td rowspan="2"><i><b>vyper</b><br>780 contracts<br>21244 functions</i></td>
  <td><i>errors:</i></td>
  <td>52.3%, 11103 :1st_place_medal:</td>
  <td>100.0%, 21244</td>
  <td>56.8%, 12077</td>
 </tr>
 <tr>
  <td><i>Time:</i></td>
  <td>7.77s (16.05s)</td>
  <td>10.24s</td>
  <td>0.98s</td>
 </tr>
</table>

See [benchmark/README.md](./benchmark/) for the methodology and commands to reproduce these results

## How it works

Short: Executes code with a custom EVM and traces CALLDATA usage.

Long: TODO

## License
MIT

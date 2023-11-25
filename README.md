# EVMole

[![PyPI](https://img.shields.io/pypi/v/evmole)](https://pypi.org/project/evmole)
[![npm](https://img.shields.io/npm/v/evmole)](https://www.npmjs.com/package/evmole)
[![license](https://img.shields.io/github/license/cdump/evmole)](./LICENSE)

Extracts [function selectors](https://docs.soliditylang.org/en/latest/abi-spec.html#function-selector) from EVM bytecode, even for unverified contracts.

- Python & JavaScript implementations
- Clean code with zero dependencies
- [Faster and more accurate](#Benchmark) than other tools
- Tested on Solidity and Vyper compiled contracts

## Usage

### JavaScript
```sh
$ npm i evmole
```
```javascript
import {functionSelectors} from 'evmole'

const code = '0x6080604052600436106025575f3560e01c8063b69ef8a8146029578063d0e30db014604d575b5f80fd5b3480156033575f80fd5b50603b5f5481565b60405190815260200160405180910390f35b60536055565b005b345f8082825460639190606a565b9091555050565b80820180821115608857634e487b7160e01b5f52601160045260245ffd5b9291505056fea2646970667358221220354240f63068d555e9b817619001b0dff6ea630d137edc1a640dae8e3ebb959864736f6c63430008170033'
console.log( functionSelectors(code) )
// Output(list): [ 'b69ef8a8', 'd0e30db0' ]
```

### Python
```sh
$ pip install evmole --upgrade
```
```python
from evmole import function_selectors

code = '0x6080604052600436106025575f3560e01c8063b69ef8a8146029578063d0e30db014604d575b5f80fd5b3480156033575f80fd5b50603b5f5481565b60405190815260200160405180910390f35b60536055565b005b345f8082825460639190606a565b9091555050565b80820180821115608857634e487b7160e01b5f52601160045260245ffd5b9291505056fea2646970667358221220354240f63068d555e9b817619001b0dff6ea630d137edc1a640dae8e3ebb959864736f6c63430008170033'
print( function_selectors(code) )
# Output(list): ['b69ef8a8', 'd0e30db0']
```

See [examples](./examples) for more

## Benchmark

<i>FP/FN</i> - [False Positive/False Negative](https://en.wikipedia.org/wiki/False_positives_and_false_negatives) errors; smaller is better

<table>
 <tr>
  <td>Dataset</td>
  <td></td>
  <td><a href="benchmark/providers/simple/"><b><i>simple</i></b></a></td>
  <td><a href="benchmark/providers/whatsabi/"><b><i>whatsabi</i></b></a></td>
  <td><a href="benchmark/providers/evmole-js/"><b><i>evmole-js</i></b></a> (<a href="benchmark/providers/evmole-py/"><b><i>py</i></b></a>)</td>
 </tr>
 <tr>
 <td rowspan="3"><i><b>largest1k</b><br>1000 contracts<br>24427 functions</i></td>
  <td><i>FP/FN contracts:</i></td>
  <td>95 / 9</td>
  <td>38 / 8</td>
  <td>1 / 0 :1st_place_medal:</td>
 </tr>
 <tr>
  <td><i>FP/FN functions:</i></td>
  <td>749 / 12</td>
  <td>38 / 8 :1st_place_medal: :2nd_place_medal:</td>
  <td>192 / 0 :2nd_place_medal: :1st_place_medal:</td>
 </tr>
 <tr>
  <td><i>Time:</i></td>
  <td>2.06s</td>
  <td>3.8s</td>
  <td>1.99s (2.09s) :rocket:</td>
 </tr>
 <tr><td colspan="6"></td></tr>
 <tr>
 <td rowspan="3"><i><b>random50k</b><br>50000 contracts<br>1171102 functions</i></td>
  <td><i>FP/FN contracts:</i></td>
  <td>4136 / 77</td>
  <td>251 / 31</td>
  <td>1 / 9 :1st_place_medal:</td>
 </tr>
 <tr>
  <td><i>FP/FN functions:</i></td>
  <td>14652 / 96</td>
  <td>261 / 32</td>
  <td>3 / 10 :1st_place_medal:</td>
 </tr>
 <tr>
  <td><i>Time:</i></td>
  <td>32.3s</td>
  <td>71.13s</td>
  <td>25.63s (33.56s) :rocket:</td>
 </tr>
 <tr><td colspan="6"></td></tr>
 <tr>
 <td rowspan="3"><i><b>vyper</b><br>780 contracts<br>21244 functions</i></td>
  <td><i>FP/FN contracts:</i></td>
  <td>185 / 480</td>
  <td>178 / 780</td>
  <td>0 / 0 :1st_place_medal:</td>
 </tr>
 <tr>
  <td><i>FP/FN functions:</i></td>
  <td>197 / 12971</td>
  <td>181 / 21244</td>
  <td>0 / 0 :1st_place_medal:</td>
 </tr>
 <tr>
  <td><i>Time:</i></td>
  <td>1.71s</td>
  <td>2.52s</td>
  <td>1.58s (1.8s) :rocket:</td>
 </tr>
</table>

See [benchmark/README.md](./benchmark/) for the methodology and commands to reproduce these results

## How it works

Short: Executes code with a custom EVM and traces CALLDATA usage.

Long: TODO

## License
MIT

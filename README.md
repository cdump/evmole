![EVMole](./.github/logo.svg)

[![try it online](https://img.shields.io/badge/Try_It_Online-evmole.xyz-brightgreen)](https://evmole.xyz/)
[![npm](https://img.shields.io/npm/v/evmole)](https://www.npmjs.com/package/evmole)
[![Crates.io](https://img.shields.io/crates/v/evmole?color=e9b44f)](https://crates.io/crates/evmole)
[![PyPI](https://img.shields.io/pypi/v/evmole?color=006dad)](https://pypi.org/project/evmole)
[![Go](https://img.shields.io/badge/go-pkg-00ADD8)](https://pkg.go.dev/github.com/cdump/evmole/go)

EVMole is a powerful library that extracts information from Ethereum Virtual Machine (EVM) bytecode, including [function selectors](https://docs.soliditylang.org/en/latest/abi-spec.html#function-selector), arguments, [state mutability](https://docs.soliditylang.org/en/latest/contracts.html#state-mutability), persistent and transient storage layouts, and CBOR metadata, even for unverified contracts.

## Key Features

- Multi-language support: Available as [JavaScript](#javascript), [Rust](#rust), [Python](#python), and [Go](#go) libraries.
- High accuracy and performance: [Outperforms](#benchmark) existing tools.
- Broad compatibility: Tested with both Solidity and Vyper compiled contracts.
- Lightweight: Clean codebase with minimal external dependencies.
- Unverified contract analysis: Extracts information even from unverified bytecode.
- Selector dispatch classification: Distinguishes normal ABI dispatch from selectors handled by fallback logic.
- CBOR metadata: Extracts string-keyed values from a terminal, length-suffixed CBOR map without assuming a particular compiler.


## Usage
### JavaScript
[API documentation](./javascript/#api) and [usage examples](./javascript/#usage) (Node.js, Vite, webpack, Parcel, esbuild)
```sh
npm i evmole
```
```javascript
import { contractInfo } from 'evmole'

const code = '0x6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256'

console.log( contractInfo(code, {selectors:true, arguments:true, stateMutability:true}) )
// {
//   functions: [
//     {
//       selector: '2125b65b',
//       bytecodeOffset: 52,
//       dispatch: 'abi',
//       arguments: 'uint32,address,uint224',
//       stateMutability: 'pure'
//     },
//     ...
```

### Rust
Documentation is available on [docs.rs](https://docs.rs/evmole/latest/evmole/)
```rust
let code = hex::decode("6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256").unwrap();

println!("{:?}", evmole::contract_info(
    evmole::ContractInfoArgs::new(&code)
        .with_selectors()
        .with_arguments()
        .with_state_mutability()
    )
);
// Contract {
//     functions: Some([
//         Function {
//             selector: [33, 37, 182, 91],
//             bytecode_offset: 52,
//             dispatch: Abi,
//             arguments: Some([Uint(32), Address, Uint(224)]),
//             state_mutability: Some(Pure)
//         },
//         ...
```

### Python
[API documentation](./python/#api)
```sh
pip install evmole --upgrade
```
```python
from evmole import contract_info

code = '0x6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256'

print( contract_info(code, selectors=True, arguments=True, state_mutability=True) )
# Contract(
#     functions=[
#     Function(
#             selector=2125b65b,
#             bytecode_offset=52,
#             dispatch="abi",
#             arguments=uint32,address,uint224,
#             state_mutability=pure),
#     ...
```

### Go
[API documentation](./go/#api-reference)
```sh
go get github.com/cdump/evmole/go
```
```go
package main

import (
    "context"
    "encoding/hex"
    "fmt"

    "github.com/cdump/evmole/go"
)

func main() {
    code, _ := hex.DecodeString("6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256")

    info, _ := evmole.ContractInfo(context.Background(), code, evmole.Options{
        Selectors:       true,
        Arguments:       true,
        StateMutability: true,
    })

    for _, fn := range info.Functions {
        fmt.Printf("%s: %s @ %d\n", fn.Selector, *fn.Arguments, fn.BytecodeOffset)
    }
    // 2125b65b: uint32,address,uint224 @ 52
    // b69ef8a8:  @ 68
}
```

### Foundry
<a href="https://getfoundry.sh/">Foundry's cast</a> uses the Rust implementation of EVMole
```sh

$ cast selectors $(cast code 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2)
0x06fdde03                           view
0x095ea7b3  address,uint256          nonpayable
0x18160ddd                           view
0x23b872dd  address,address,uint256  nonpayable
...

$ cast selectors --resolve $(cast code 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2)
0x06fdde03                           view        name()
0x095ea7b3  address,uint256          nonpayable  approve(address,uint256)
0x18160ddd                           view        totalSupply()
0x23b872dd  address,address,uint256  nonpayable  transferFrom(address,address,uint256)
...
```

### AI agents

For application code, use one of the language bindings above. For agent-driven
bytecode analysis, choose one integration.

#### JSON CLI

Use for one-off analysis and scripts:

```bash
npx -y evmole analyze --bytecode 0x...
```

#### Portable skill

Install routing and interpretation guidance for supported agents:

```bash
npx skills add cdump/evmole --skill evm-bytecode-analysis -g
```

#### Local MCP server

Expose EVMole as a typed local tool:

```bash
npx -y evmole-mcp
```

All integrations expect deployed/runtime bytecode and run locally without
sending bytecode to an EVMole-operated service. See the
[agent integration guide](./agent/README.md) for setup, schemas,
limitations, and privacy details.


## Benchmark

### function selectors
<i>FP/FN</i> - [False Positive/False Negative](https://en.wikipedia.org/wiki/False_positives_and_false_negatives) errors; <b>smaller is better</b>

<table>
 <tr>
  <td>Dataset</td>
  <td></td>
  <td><b><i>evmole</i></b> <a href="benchmark/providers/evmole-rs/"><b><i>rs</i></b></a> · <a href="benchmark/providers/evmole-js/"><b><i>js</i></b></a> · <a href="benchmark/providers/evmole-py/"><b><i>py</i></b></a> · <a href="benchmark/providers/evmole-go/"><b><i>go</i></b></a></td>
  <td><a href="benchmark/providers/whatsabi/"><b><i>whatsabi</i></b></a></td>
  <td><a href="benchmark/providers/sevm/"><b><i>sevm</i></b></a></td>
  <td><a href="benchmark/providers/evm-hound-rs/"><b><i>evmhound</i></b></a></td>
  <td><a href="benchmark/providers/heimdall-rs/"><b><i>heimdall</i></b></a></td>
 </tr>
 <tr>
  <td rowspan="5"><b>coverage2k</b><br><sub>solidity<br><br>2000<br>addresses<br><br>45650<br>functions</sub></td>
  <td><i>FP <sub>addrs</sub></i></td>
  <td>0 🥇</td>
  <td>4</td>
  <td>1</td>
  <td>56</td>
  <td>3</td>
 </tr>
 <tr>
  <td><i>FN <sub>addrs</sub></i></td>
  <td>0 🥇</td>
  <td>11</td>
  <td>0 🥇</td>
  <td>224</td>
  <td>166</td>
 </tr>
 <tr>
  <td><i>FP <sub>funcs</sub></i></td>
  <td>0 🥇</td>
  <td>10</td>
  <td>10</td>
  <td>616</td>
  <td>27</td>
 </tr>
 <tr>
  <td><i>FN <sub>funcs</sub></i></td>
  <td>0 🥇</td>
  <td>145</td>
  <td>0 🥇</td>
  <td>821</td>
  <td>342</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>24ms · 0.3s · 31ms · 0.1s</td>
  <td>2.0s</td>
  <td>28s<sup>(*)</sup></td>
  <td>86ms</td>
  <td>111s<sup>(*)</sup></td>
 </tr>
 <tr><td colspan="7"></td></tr>
 <tr>
  <td rowspan="5"><b>random10k</b><br><sub>solidity<br><br>10000<br>addresses<br><br>223316<br>functions</sub></td>
  <td><i>FP <sub>addrs</sub></i></td>
  <td>0 🥇</td>
  <td>19</td>
  <td>16</td>
  <td>224</td>
  <td>7</td>
 </tr>
 <tr>
  <td><i>FN <sub>addrs</sub></i></td>
  <td>0 🥇</td>
  <td>44</td>
  <td>1</td>
  <td>838</td>
  <td>819</td>
 </tr>
 <tr>
  <td><i>FP <sub>funcs</sub></i></td>
  <td>0 🥇</td>
  <td>65</td>
  <td>112</td>
  <td>3157</td>
  <td>260</td>
 </tr>
 <tr>
  <td><i>FN <sub>funcs</sub></i></td>
  <td>0 🥇</td>
  <td>173</td>
  <td>7</td>
  <td>4112</td>
  <td>1021</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>0.1s · 0.8s · 0.2s · 0.8s</td>
  <td>7.1s</td>
  <td>80s<sup>(*)</sup></td>
  <td>0.4s</td>
  <td>533s<sup>(*)</sup></td>
 </tr>
 <tr><td colspan="7"></td></tr>
 <tr>
  <td rowspan="5"><b>coverage1k</b><br><sub>vyper<br><br>1000<br>addresses<br><br>38759<br>functions</sub></td>
  <td><i>FP <sub>addrs</sub></i></td>
  <td>0 🥇</td>
  <td>560</td>
  <td>0 🥇</td>
  <td>2</td>
  <td>0 🥇</td>
 </tr>
 <tr>
  <td><i>FN <sub>addrs</sub></i></td>
  <td>0 🥇</td>
  <td>998</td>
  <td>788</td>
  <td>525</td>
  <td>998</td>
 </tr>
 <tr>
  <td><i>FP <sub>funcs</sub></i></td>
  <td>0 🥇</td>
  <td>560</td>
  <td>0 🥇</td>
  <td>5</td>
  <td>0 🥇</td>
 </tr>
 <tr>
  <td><i>FN <sub>funcs</sub></i></td>
  <td>0 🥇</td>
  <td>38759</td>
  <td>34077</td>
  <td>16218</td>
  <td>38759</td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>91ms · 0.4s · 0.1s · 0.3s</td>
  <td>1.9s</td>
  <td>5.4s<sup>(*)</sup></td>
  <td>67ms</td>
  <td>12s<sup>(*)</sup></td>
 </tr>
</table>

### function arguments
<i>Errors</i> - when at least 1 inferred argument is incorrect: `(uint256,string)` ≠ `(uint256,bytes)`

<table>
 <tr>
  <td>Dataset</td>
  <td></td>
  <td><b><i>evmole</i></b> <a href="benchmark/providers/evmole-rs/"><b><i>rs</i></b></a> · <a href="benchmark/providers/evmole-js/"><b><i>js</i></b></a> · <a href="benchmark/providers/evmole-py/"><b><i>py</i></b></a> · <a href="benchmark/providers/evmole-go/"><b><i>go</i></b></a></td>
  <td><a href="benchmark/providers/heimdall-rs/"><b><i>heimdall</i></b></a></td>
 </tr>
 <tr>
  <td rowspan="2"><b>coverage2k</b><br><sub>solidity<br><br>45650<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>8.5% 🥇<br><sub>3883</sub></td>
  <td>23.3%<br><sub>10643</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>0.8s · 1.7s · 1.1s · 2.1s</td>
  <td>111s<sup>(*)</sup></td>
 </tr>
 <tr><td colspan="4"></td></tr>
 <tr>
  <td rowspan="2"><b>random10k</b><br><sub>solidity<br><br>223316<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>6.8% 🥇<br><sub>15296</sub></td>
  <td>21.4%<br><sub>47878</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>3.5s · 6.9s · 4.2s · 9.7s</td>
  <td>511s<sup>(*)</sup></td>
 </tr>
 <tr><td colspan="4"></td></tr>
 <tr>
  <td rowspan="2"><b>coverage1k</b><br><sub>vyper<br><br>38759<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>45.4% 🥇<br><sub>17590</sub></td>
  <td>100.0%<br><sub>38759</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>0.5s · 1.1s · 0.6s · 1.4s</td>
  <td>12s<sup>(*)</sup></td>
 </tr>
</table>

### function state mutability

<i>Errors</i> - Results are not equal (treating `view` and `pure` as equivalent to `nonpayable`)

<i>Errors strict</i> - Results are strictly unequal (`nonpayable` ≠ `view`). Some ABIs mark `pure`/`view` functions as `nonpayable`, so not all strict errors indicate real issues.

<table>
 <tr>
  <td>Dataset</td>
  <td></td>
  <td><b><i>evmole</i></b> <a href="benchmark/providers/evmole-rs/"><b><i>rs</i></b></a> · <a href="benchmark/providers/evmole-js/"><b><i>js</i></b></a> · <a href="benchmark/providers/evmole-py/"><b><i>py</i></b></a> · <a href="benchmark/providers/evmole-go/"><b><i>go</i></b></a></td>
  <td><a href="benchmark/providers/whatsabi/"><b><i>whatsabi</i></b></a></td>
  <td><a href="benchmark/providers/sevm/"><b><i>sevm</i></b></a></td>
  <td><a href="benchmark/providers/heimdall-rs/"><b><i>heimdall</i></b></a></td>
 </tr>
 <tr>
  <td rowspan="3"><b>coverage2k</b><br><sub>solidity<br><br>45647<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>0.0% 🥇<br><sub>18</sub></td>
  <td>52.2%<br><sub>23810</sub></td>
  <td>11.2%<br><sub>5133</sub></td>
  <td>19.6%<br><sub>8951</sub></td>
 </tr>
 <tr>
  <td><i>Errors strict</i></td>
  <td>9.0% 🥇<br><sub>4107</sub></td>
  <td>70.9%<br><sub>32346</sub></td>
  <td>62.3%<br><sub>28429</sub></td>
  <td>41.3%<br><sub>18835</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>10s · 11s · 9.7s · 20s</td>
  <td>3.1s</td>
  <td>29s<sup>(*)</sup></td>
  <td>112s<sup>(*)</sup></td>
 </tr>
 <tr><td colspan="6"></td></tr>
 <tr>
  <td rowspan="3"><b>random10k</b><br><sub>solidity<br><br>223273<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>0.0% 🥇<br><sub>39</sub></td>
  <td>48.8%<br><sub>108928</sub></td>
  <td>9.3%<br><sub>20713</sub></td>
  <td>18.6%<br><sub>41507</sub></td>
 </tr>
 <tr>
  <td><i>Errors strict</i></td>
  <td>8.9% 🥇<br><sub>19940</sub></td>
  <td>69.9%<br><sub>156081</sub></td>
  <td>60.9%<br><sub>136069</sub></td>
  <td>40.1%<br><sub>89519</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>48s · 50s · 46s · 88s</td>
  <td>12s</td>
  <td>81s<sup>(*)</sup></td>
  <td>512s<sup>(*)</sup></td>
 </tr>
 <tr><td colspan="6"></td></tr>
 <tr>
  <td rowspan="3"><b>coverage1k</b><br><sub>vyper<br><br>38278<br>functions</sub></td>
  <td><i>Errors</i></td>
  <td>0.1% 🥇<br><sub>34</sub></td>
  <td>100.0%<br><sub>38278</sub></td>
  <td>96.2%<br><sub>36814</sub></td>
  <td>100.0%<br><sub>38278</sub></td>
 </tr>
 <tr>
  <td><i>Errors strict</i></td>
  <td>1.2% 🥇<br><sub>441</sub></td>
  <td>100.0%<br><sub>38278</sub></td>
  <td>98.4%<br><sub>37650</sub></td>
  <td>100.0%<br><sub>38278</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>45s · 39s · 44s · 89s</td>
  <td>2.0s</td>
  <td>5.5s<sup>(*)</sup></td>
  <td>12s<sup>(*)</sup></td>
 </tr>
</table>

### Control Flow Graph

<i>False Negatives</i> - Valid blocks possibly incorrectly marked unreachable by CFG analysis. Lower count usually indicates better precision.

<table>
 <tr>
  <td></td>
  <td><b><i>evmole</i></b> <a href="benchmark/providers/evmole-rs/"><b><i>rs</i></b></a> · <a href="benchmark/providers/evmole-js/"><b><i>js</i></b></a> · <a href="benchmark/providers/evmole-py/"><b><i>py</i></b></a> · <a href="benchmark/providers/evmole-go/"><b><i>go</i></b></a></td>
  <td><a href="benchmark/providers/ethersolve"><b><i>ethersolve</i></b></a></td>
  <td><a href="benchmark/providers/evm-cfg"><b><i>evm-cfg</i></b></a></td>
  <td><a href="benchmark/providers/sevm"><b><i>sevm</i></b></a></td>
  <td><a href="benchmark/providers/heimdall-rs"><b><i>heimdall-rs</i></b></a></td>
  <td><a href="benchmark/providers/evm-cfg-builder"><b><i>evm-cfg-builder</i></b></a></td>
 </tr>
 <tr>
  <td><i>Basic Blocks</i></td>
  <td>92.8% 🥇<br><sub>483212</sub></td>
  <td>52.5%<br><sub>273518</sub></td>
  <td>58.6%<br><sub>305248</sub></td>
  <td>37.3%<br><sub>194368</sub></td>
  <td>32.6%<br><sub>169980</sub></td>
  <td>14.5%<br><sub>75383</sub></td>
 </tr>
 <tr>
  <td><i>False Negatives</i></td>
  <td>7.2% 🥇<br><sub>37496</sub></td>
  <td>47.5%<br><sub>247190</sub></td>
  <td>41.4%<br><sub>215460</sub></td>
  <td>62.7%<br><sub>326340</sub></td>
  <td>67.4%<br><sub>350728</sub></td>
  <td>85.5%<br><sub>445325</sub></td>
 </tr>
 <tr>
  <td><i>Time</i></td>
  <td>14s · 26s · 12s · 55s</td>
  <td>888s</td>
  <td>36s</td>
  <td>9.8s</td>
  <td>20s</td>
  <td>359s</td>
 </tr>
</table>

dataset flow-challenge500, 500 contracts, 520,708 blocks

### notes

See [benchmark/README.md](./benchmark/) for the methodology and commands to reproduce these results

<i>versions: evmole v0.9.3; <a href="https://github.com/shazow/whatsabi">whatsabi</a> v0.25.0; <a href="https://github.com/acuarica/evm">sevm</a> v0.7.4; <a href="https://github.com/g00dv1n/evm-hound-rs">evm-hound-rs</a> v0.1.4; <a href="https://github.com/Jon-Becker/heimdall-rs">heimdall-rs</a> v0.9.3</i>

<sup>(*)</sup>: <b>sevm</b> and <b>heimdall-rs</b> are full decompilers, not limited to extracting function selectors

## How it works

EVMole uses symbolic execution with a custom EVM implementation to trace how CALLDATA flows through the bytecode:

This approach is more accurate than static pattern matching because it follows the actual execution paths the EVM would take, correctly handling complex dispatchers, proxy patterns, and compiler-specific optimizations from both Solidity and Vyper.

## Talks
- [EVMole: function selectors and arguments from bytecode](https://www.youtube.com/watch?v=l0udabGej54) - BlockSplit 2024
- [EVMole: function selectors and arguments from bytecode](https://ethcc.io/archives/evmole-function-selectors-and-arguments-from-bytecode) - EthCC 2024
- [Reconstructing Control Flow Graphs from EVM Bytecode](https://www.youtube.com/watch?v=1Xd6PhEHMHM) - ETHTaipei 2025
- [Reconstructing Control Flow Graphs from EVM Bytecode: Faster, Better, Stronger](https://www.youtube.com/watch?v=UL6-3EZbv3E) - EthCC 2025

## License
MIT

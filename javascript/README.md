# EVMole JavaScript (WASM)

This directory contains [API documentation](#api) and various examples ([web page](#web-page), [nodejs](#nodejs), [vite](#vite), [webpack](#webpack), [parcel](#parcel), [esbuild](#esbuild)) demonstrating how to use the EVMole library with its JavaScript (WASM) build in different environments and with various build tools.

The library is built with [wasm-pack](https://rustwasm.github.io/wasm-pack/). To simplify usage, we provide a  [default entry point](./src/evmole_esm.js) with `await init()`, which should work in [all modern browsers](https://caniuse.com/mdn-javascript_operators_await_top_level) and bundlers.


## Usage
### Web page

You can load evmole directly in a web page using a script module. Here's how to do it:
<!--
TODO: fix jsdelivr esm import
import { functionSelectors } from 'https://cdn.jsdelivr.net/npm/evmole/+esm';
-->
```html
<div id="info"></div>

<script type="module">
import { contractInfo } from 'https://cdn.jsdelivr.net/npm/evmole@0.6.2/dist/evmole.mjs';

const bytecode = '0x6080...'; // Replace with actual bytecode
document.getElementById('info').textContent = contractInfo(bytecode, {selectors: true, arguments: true, stateMutability: true});
</script>
```

### Node.js

You can use EVMole with both import and require syntax:
- [with import](./examples/node/with_import.mjs)
- [with require](./examples/node/with_require.cjs)

### Vite

Set `target: esnext` in [vite.config.js](./examples/vite/vite.config.js) to support Top Level Await, required for default EVMole import:
```javascript
build: {
  target: 'esnext'
}
```

After that, [import and use EVMole](./examples/vite/main.js) as usual.

If you can't use `esnext`, see the [No Top Level Await](#no-top-level-await) section.


### Webpack

Set `asyncWebAssembly: true` in [webpack.config.js](./examples/webpack/webpack.config.js):
```javascript
experiments: {
  asyncWebAssembly: true,
}
```
After that, [import and use EVMole](./examples/webpack/index.js) as usual.

### Parcel

Parcel can't work with Top Level Await, so you need to manually call init after import. See examples with:
- [default parcel installation](./examples/parcel/src/app.js)
- [example](./examples/parcel_packageExports/src/app.js) with `"packageExports": true` set in [package.json](./examples/parcel_packageExports/package.json)

You can read more about this in [parcel resolver documentation](https://parceljs.org/blog/v2-9-0/#new-resolver)


### esbuild

Pass `--format=esm` and `--loader:.wasm=file` to esbuild.
Find the [full command in package.json](./examples/esbuild/package.json)

After that, [import and use EVMole](./examples/esbuild/main.js) as usual.

### No Top Level Await
If you can't use [Top Level Await](https://caniuse.com/mdn-javascript_operators_await_top_level), you can import EVMole as:

```js
import init, { contractInfo } from 'evmole/no_tla`
// or: from 'evmole/dist/evmole.js' (supported, but not recommended)
```

After that, you can use it as:
```javascript
const bytecode = '0x6080...'; // Replace with actual bytecode
async function main() {
  await init();
  console.log(contractInfo(bytecode, {selectors: true}));
}
main()
```
or
```javascript
const bytecode = '0x6080...'; // Replace with actual bytecode
init().then() => {
  console.log(contractInfo(bytecode, {selectors: true}));
}
```

See full example without Top Level Await in [Parcel example](./examples/parcel/src/app.js)

<!-- generated with `npm run doc` -->
### API

<a name="contractInfo"></a>

### contractInfo(code, args) ⇒ [<code>Contract</code>](#Contract)
Analyzes contract bytecode and returns contract information based on specified options.

**Kind**: global function  
**Returns**: [<code>Contract</code>](#Contract) - Analyzed contract information  

| Param | Type | Description |
| --- | --- | --- |
| code | <code>string</code> | Runtime bytecode as a hex string |
| args | <code>Object</code> | Configuration options for the analysis |
| [args.selectors] | <code>boolean</code> | When true, includes function selectors in the output |
| [args.arguments] | <code>boolean</code> | When true, includes function arguments information |
| [args.state_mutability] | <code>boolean</code> | When true, includes state mutability information for functions |
| [args.storage] | <code>boolean</code> | When true, includes contract storage layout information |


<a name="Contract"></a>

### Contract : <code>Object</code>
**Kind**: global typedef  
**Properties**

| Name | Type | Description |
| --- | --- | --- |
| [functions] | [<code>Array.&lt;ContractFunction&gt;</code>](#ContractFunction) | Array of functions found in the contract. Not present if no functions were extracted |
| [storage] | [<code>Array.&lt;StorageRecord&gt;</code>](#StorageRecord) | Array of storage records found in the contract. Not present if storage layout was not extracted |


<a name="ContractFunction"></a>

### ContractFunction : <code>Object</code>
**Kind**: global typedef  
**Properties**

| Name | Type | Description |
| --- | --- | --- |
| selector | <code>string</code> | Function selector as a 4-byte hex string without '0x' prefix (e.g., 'aabbccdd') |
| bytecode_offset | <code>number</code> | Starting byte offset within the EVM bytecode for the function body |
| [arguments] | <code>string</code> | Function argument types in canonical format (e.g., 'uint256,address[]'). Not present if arguments were not extracted |
| [state_mutability] | <code>string</code> | Function's state mutability ("pure", "view", "payable", or "nonpayable"). Not present if state mutability were not extracted |

<a name="StorageRecord"></a>


### StorageRecord : <code>Object</code>
**Kind**: global typedef  
**Properties**

| Name | Type | Description |
| --- | --- | --- |
| slot | <code>string</code> | Storage slot number as a hex string (e.g., '0', '1b') |
| offset | <code>number</code> | Byte offset within the storage slot (0-31) |
| type | <code>string</code> | Variable type (e.g., 'uint256', 'mapping(address => uint256)', 'bytes32') |
| reads | <code>Array.&lt;string&gt;</code> | Array of function selectors that read from this storage location |
| writes | <code>Array.&lt;string&gt;</code> | Array of function selectors that write to this storage location |

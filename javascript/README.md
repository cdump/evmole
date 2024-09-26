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
<div id="selectors"></div>

<script type="module">
import { functionSelectors } from 'https://cdn.jsdelivr.net/npm/evmole@0.5.1/dist/evmole.mjs';

const bytecode = '0x6080...'; // Replace with actual bytecode
document.getElementById('selectors').textContent = functionSelectors(bytecode);
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
import init, { functionSelectors } from 'evmole/no_tla`
// or: from 'evmole/dist/evmole.js' (supported, but not recommended)
```

After that, you can use it as:
```javascript
const bytecode = '0x6080...'; // Replace with actual bytecode
async function main() {
  await init();
  console.log(functionSelectors(bytecode));
}
main()
```
or
```javascript
const bytecode = '0x6080...'; // Replace with actual bytecode
init().then() => {
  console.log(functionSelectors(bytecode));
}
```

See full example without Top Level Await in [Parcel example](./examples/parcel/src/app.js)

<!-- generated with `npm run doc` -->
### API

<dl>
<dt><a href="#functionSelectors">functionSelectors(code, gas_limit)</a> ⇒ <code>Array.&lt;string&gt;</code></dt>
<dd><p>Extracts function selectors from the given bytecode.</p>
</dd>
<dt><a href="#functionArguments">functionArguments(code, selector, gas_limit)</a> ⇒ <code>string</code></dt>
<dd><p>Extracts function arguments for a given selector from the bytecode.</p>
</dd>
<dt><a href="#functionStateMutability">functionStateMutability(code, selector, gas_limit)</a> ⇒ <code>string</code></dt>
<dd><p>Extracts function state mutability for a given selector from the bytecode.</p>
</dd>
</dl>

<a name="functionSelectors"></a>

### functionSelectors(code, gas_limit) ⇒ <code>Array.&lt;string&gt;</code>
Extracts function selectors from the given bytecode.

**Returns**: <code>Array.&lt;string&gt;</code> - Function selectors as a hex strings

| Param | Type | Description |
| --- | --- | --- |
| code | <code>string</code> | Runtime bytecode as a hex string |
| gas_limit | <code>number</code> | Maximum allowed gas usage; set to `0` to use defaults |

<a name="functionArguments"></a>

### functionArguments(code, selector, gas_limit) ⇒ <code>string</code>
Extracts function arguments for a given selector from the bytecode.

**Returns**: <code>string</code> - Function arguments (ex: 'uint32,address')

| Param | Type | Description |
| --- | --- | --- |
| code | <code>string</code> | Runtime bytecode as a hex string |
| selector | <code>string</code> | Function selector as a hex string |
| gas_limit | <code>number</code> | Maximum allowed gas usage; set to `0` to use defaults |

<a name="functionStateMutability"></a>

### functionStateMutability(code, selector, gas_limit) ⇒ <code>string</code>
Extracts function state mutability for a given selector from the bytecode.

**Returns**: <code>string</code> - `payable` | `nonpayable` | `view` | `pure`

| Param | Type | Description |
| --- | --- | --- |
| code | <code>string</code> | Runtime bytecode as a hex string |
| selector | <code>string</code> | Function selector as a hex string |
| gas_limit | <code>number</code> | Maximum allowed gas usage; set to `0` to use defaults |

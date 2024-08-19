# EVMole JavaScript (WASM)

This directory contains various examples demonstrating how to use the EVMole library with its JavaScript (WASM) build in different environments and with various build tools.

The library is built with [wasm-pack](https://rustwasm.github.io/wasm-pack/). To simplify usage, we provide a  [default entry point](./src/evmole_esm.js) with `await init()`, which should work in [all modern browsers](https://caniuse.com/mdn-javascript_operators_await_top_level) and bundlers.

## Web page

You can load evmole directly in a web page using a script module. Here's how to do it:
```html
<div id="selectors"></div>

<script type="module">
import { functionArguments, functionSelectors } from 'https://cdn.jsdelivr.net/npm/evmole/+esm';

const bytecode = '0x6080...'; // Replace with actual bytecode
document.getElementById('selectors').textContent = functionSelectors(bytecode);
</script>
```

## Node.js

You can use EVMole with both import and require syntax:
- [with import](./examples/node/with_import.mjs)
- [with require](./examples/node/with_require.cjs)

## Vite

Set `target: esnext` in [vite.config.js](./examples/vite/vite.config.js) to support Top Level Await, required for default EVMole import:
```javascript
build: {
  target: 'esnext'
}
```

After that, [import and use EVMole](./examples/vite/main.js) as usual.

If you can't use `esnext`, see the [No Top Level Await](#no-top-level-await) section.


## Webpack

Set `asyncWebAssembly: true` in [webpack.config.js](./examples/webpack/webpack.config.js):
```javascript
experiments: {
  asyncWebAssembly: true,
}
```
After that, [import and use EVMole](./examples/webpack/index.js) as usual.

## Parcel

Parcel can't work with Top Level Await, so you need to manually call init after import. See examples with:
- [default parcel installation](./examples/parcel/src/app.js)
- [example](./examples/parcel_packageExports/src/app.js) with `"packageExports": true` set in [package.json](./examples/parcel_packageExports/package.json)

You can read more about this in [parcel resolver documentation](https://parceljs.org/blog/v2-9-0/#new-resolver)


## esbuild

Pass `--format=esm` and `--loader:.wasm=file` to esbuild.
Find the [full command in package.json](./examples/esbuild/package.json)

After that, [import and use EVMole](./examples/esbuild/main.js) as usual.

## No Top Level Await
If you can't use [Top Level Await](https://caniuse.com/mdn-javascript_operators_await_top_level), you can import EVMole as:

```js
import init, { functionSelectors, functionArguments } from 'evmole/no_tla`
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

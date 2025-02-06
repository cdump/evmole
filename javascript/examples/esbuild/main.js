import { contractInfo } from 'evmole/wasm_import'

document.body.innerHTML = `
  <div id="selectors"></div>
  <br>
  <div id="arguments"></div>
  <br>
  <div id="state_mutability"></div>
  <br>
  <div id="disassembled"></div>
`

const code = '6080604052348015600e575f80fd5b50600436106026575f3560e01c8063fae7ab8214602a575b5f80fd5b603960353660046062565b6052565b60405163ffffffff909116815260200160405180910390f35b5f605c826001608a565b92915050565b5f602082840312156071575f80fd5b813563ffffffff811681146083575f80fd5b9392505050565b63ffffffff8181168382160190811115605c57634e487b7160e01b5f52601160045260245ffd';

const info = contractInfo(code, { selectors: false, arguments: true, stateMutability: true, disassemble: true });

document.getElementById('selectors').innerText = info.functions[0].selector;

document.getElementById('arguments').innerText = info.functions[0].arguments;

document.getElementById('state_mutability').innerText = info.functions[0].stateMutability;

document.getElementById('disassembled').innerText = info.disassembled.map(([pc, val]) => `${pc} ${val}`).join('\n')

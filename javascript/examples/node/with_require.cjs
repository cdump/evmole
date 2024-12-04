const assert = require('node:assert');
const evmole = require('evmole');

const code = '6080604052348015600e575f80fd5b50600436106026575f3560e01c8063fae7ab8214602a575b5f80fd5b603960353660046062565b6052565b60405163ffffffff909116815260200160405180910390f35b5f605c826001608a565b92915050565b5f602082840312156071575f80fd5b813563ffffffff811681146083575f80fd5b9392505050565b63ffffffff8181168382160190811115605c57634e487b7160e01b5f52601160045260245ffd';

const info = evmole.contractInfo(code, { selectors: false, arguments: true, stateMutability: true });
assert.equal(info.functions.length, 1);
assert.equal(info.functions[0].selector, 'fae7ab82');
assert.equal(info.functions[0].arguments, 'uint32');
assert.equal(info.functions[0].stateMutability, 'pure');

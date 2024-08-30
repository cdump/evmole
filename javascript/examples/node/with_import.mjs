import {functionArguments, functionSelectors, functionStateMutability} from 'evmole';
import {equal, deepEqual} from 'node:assert';

const code = '6080604052348015600e575f80fd5b50600436106026575f3560e01c8063fae7ab8214602a575b5f80fd5b603960353660046062565b6052565b60405163ffffffff909116815260200160405180910390f35b5f605c826001608a565b92915050565b5f602082840312156071575f80fd5b813563ffffffff811681146083575f80fd5b9392505050565b63ffffffff8181168382160190811115605c57634e487b7160e01b5f52601160045260245ffd';

const r = functionSelectors(code);
deepEqual(r, ['fae7ab82']);

const a = functionArguments(code, 'fae7ab82');
equal(a, 'uint32');

const m = functionStateMutability(code, 'fae7ab82');
equal(m, 'pure');

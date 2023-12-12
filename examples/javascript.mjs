import {functionArguments, functionSelectors} from 'evmole'

// output of `solc example.sol --bin-runtime --optimize`
const code = '0x608060405260043610610033575f3560e01c8063b69ef8a814610037578063d0e30db01461005d578063dd5d521114610067575b5f80fd5b348015610042575f80fd5b5061004b5f5481565b60405190815260200160405180910390f35b610065610086565b005b348015610072575f80fd5b506100656100813660046100bb565b61009d565b345f8082825461009691906100e5565b9091555050565b8063ffffffff165f808282546100b391906100e5565b909155505050565b5f602082840312156100cb575f80fd5b813563ffffffff811681146100de575f80fd5b9392505050565b8082018082111561010457634e487b7160e01b5f52601160045260245ffd5b9291505056fea2646970667358221220edc1fabb7470d674531e38cc4be9b6c0d826e719e05b0cd653821caeaa4e551964736f6c63430008170033'

let r;
r = functionSelectors(code)
console.log('all signatures with default gas_limit', r)

r = functionSelectors(code, 50)
console.log('only 1 signature found with so low gas limit', r)

r = functionSelectors(code, 200)
console.log('200 gas is enough for all signatures', r)

for (const sel of r) {
  const args = functionArguments(code, sel)
  console.log(` - arguments for ${sel} are (${args})`)
}

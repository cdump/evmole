from evmole import contract_info
from evmole import Contract, Function, StorageRecord

code = '6080604052348015600e575f80fd5b50600436106026575f3560e01c8063fae7ab8214602a575b5f80fd5b603960353660046062565b6052565b60405163ffffffff909116815260200160405180910390f35b5f605c826001608a565b92915050565b5f602082840312156071575f80fd5b813563ffffffff811681146083575f80fd5b9392505050565b63ffffffff8181168382160190811115605c57634e487b7160e01b5f52601160045260245ffd'

info = contract_info(code, selectors=True, arguments=True, state_mutability=True, disassemble=True)
assert isinstance(info, Contract)
assert info.functions is not None
assert len(info.functions) == 1
assert isinstance(info.functions[0], Function)
assert info.functions[0].selector == 'fae7ab82'
assert info.functions[0].arguments == 'uint32'
assert info.functions[0].state_mutability == 'pure'

assert info.disassembled is not None
assert info.disassembled[0] == (0, 'PUSH1 80')

print(f'Success #1, {info}')

from evmole import ControlFlowGraph, Block, BlockType, DynamicJump
info = contract_info(code, basic_blocks=True, control_flow_graph=True, selectors=True)
assert isinstance(info.basic_blocks, list)
assert isinstance(info.basic_blocks[0], tuple)

assert isinstance(info.control_flow_graph, ControlFlowGraph)
assert isinstance(info.control_flow_graph.blocks, list)
assert isinstance(info.control_flow_graph.blocks[0], Block)

b = info.control_flow_graph.blocks[0]
assert isinstance(b.btype, BlockType)
assert isinstance(b.btype, BlockType.Jumpi)
assert isinstance(b.btype.true_to, int)

print(f'Success #2, {info}')

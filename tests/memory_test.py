from evmole.evm.memory import Memory

def test_memory():
    m = Memory()

    m.store(0, b'\xaa')
    assert m.load(0) == (b'\xaa' + b'\x00'*31, [b'\xaa'])
    assert m.load(1) == (b'\x00'*32, [])

    m.store(8, b'\xbb')
    assert m.load(0) == (b'\xaa' + b'\x00'*7 + b'\xbb' + b'\x00'*23, [b'\xaa', b'\xbb'])
    assert m.load(2) == (b'\x00'*6 + b'\xbb' + b'\x00'*25, [b'\xbb'])
    assert m.load(8) == (b'\xbb' + b'\x00'*31, [b'\xbb'])
    assert m.load(9) == (b'\x00'*32, [])

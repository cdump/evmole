from evmole.evm.memory import Memory

def test_memory():
    m = Memory()

    m.store(0, b'\xaa')
    assert m.load(0) == (b'\xaa' + b'\x00'*31, {b'\xaa'})
    assert m.load(1) == (b'\x00'*32, set())

    m.store(8, b'\xbb')
    assert m.load(0) == (b'\xaa' + b'\x00'*7 + b'\xbb' + b'\x00'*23, {b'\xaa', b'\xbb'})
    assert m.load(2) == (b'\x00'*6 + b'\xbb' + b'\x00'*25, {b'\xbb'})
    assert m.load(8) == (b'\xbb' + b'\x00'*31, {b'\xbb'})
    assert m.load(9) == (b'\x00'*32, set())

    m.store(0, b'\xcc')
    assert m.load(0) == (b'\xcc' + b'\x00'*7 + b'\xbb' + b'\x00'*23, {b'\xcc', b'\xbb'})


    m = Memory()
    m.store(4, b'\xaa\xbb')
    m.store(3, b'\xcc')
    assert m.load(0) == (b'\x00' * 3 + b'\xcc\xaa\xbb' + b'\x00'*26, {b'\xaa\xbb', b'\xcc'})

    m.store(3, b'\xdd\xee')
    assert m.load(0) == (b'\x00' * 3 + b'\xdd\xee\xbb' + b'\x00'*26, {b'\xaa\xbb', b'\xdd\xee'})


    m = Memory()
    m.store(3, b'\xaa\xbb\xcc')
    m.store(4, b'\xdd')
    assert m.load(0) == (b'\x00' * 3 + b'\xaa\xdd\xcc' + b'\x00'*26, {b'\xaa\xbb\xcc', b'\xdd'})
    m.store(2, b'\xee\xff')
    assert m.load(0) == (b'\x00' * 2 + b'\xee\xff\xdd\xcc' + b'\x00'*26, {b'\xaa\xbb\xcc', b'\xdd', b'\xee\xff'})

from evmole.evm.element import Element
from evmole.evm.memory import Memory


def test_memory():
    m = Memory()

    m.store(0, Element(b'\xaa', 'l1'))
    assert m.load(0) == (b'\xaa' + b'\x00'*31, {'l1'})
    assert m.load(1) == (b'\x00'*32, set())

    m.store(8, Element(b'\xbb', 'l2'))
    assert m.load(0) == (b'\xaa' + b'\x00'*7 + b'\xbb' + b'\x00'*23, {'l1', 'l2'})
    assert m.load(2) == (b'\x00'*6 + b'\xbb' + b'\x00'*25, {'l2'})
    assert m.load(8) == (b'\xbb' + b'\x00'*31, {'l2'})
    assert m.load(9) == (b'\x00'*32, set())

    m.store(0, Element(b'\xcc')) # without label
    assert m.load(0) == (b'\xcc' + b'\x00'*7 + b'\xbb' + b'\x00'*23, {'l2'})


    m = Memory()
    m.store(4, Element(b'\xaa\xbb', 'l1'))
    m.store(3, Element(b'\xcc', 'l2'))
    assert m.load(0) == (b'\x00' * 3 + b'\xcc\xaa\xbb' + b'\x00'*26, {'l1', 'l2'})

    m.store(3, Element(b'\xdd\xee', 'l3'))
    assert m.load(0) == (b'\x00' * 3 + b'\xdd\xee\xbb' + b'\x00'*26, {'l1', 'l3'})


    m = Memory()
    m.store(3, Element(b'\xaa\xbb\xcc', 'l1'))
    m.store(4, Element(b'\xdd', 'l2'))
    assert m.load(0) == (b'\x00' * 3 + b'\xaa\xdd\xcc' + b'\x00'*26, {'l1', 'l2'})
    m.store(2, Element(b'\xee\xff', 'l3'))
    assert m.load(0) == (b'\x00' * 2 + b'\xee\xff\xdd\xcc' + b'\x00'*26, {'l1', 'l2', 'l3'})

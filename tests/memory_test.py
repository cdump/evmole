from evmole.evm.element import Element
from evmole.evm.memory import Memory


def test_memory():
    m = Memory()

    m.store(0, Element(b'\xaa', 'l1'))
    assert m.load(0) == (Element(b'\xaa' + b'\x00'*31, None), {'l1'})
    assert m.load(1) == (Element(b'\x00'*32, None), set())

    m.store(8, Element(b'\xbb', 'l2'))
    assert m.load(0) == (Element(b'\xaa' + b'\x00'*7 + b'\xbb' + b'\x00'*23, None), {'l1', 'l2'})
    assert m.load(2) == (Element(b'\x00'*6 + b'\xbb' + b'\x00'*25, None), {'l2'})
    assert m.load(8) == (Element(b'\xbb' + b'\x00'*31, None), {'l2'})
    assert m.load(9) == (Element(b'\x00'*32, None), set())

    m.store(0, Element(b'\xcc')) # without label
    assert m.load(0) == (Element(b'\xcc' + b'\x00'*7 + b'\xbb' + b'\x00'*23, None), {'l2'})


    m = Memory()
    m.store(4, Element(b'\xaa\xbb', 'l1'))
    m.store(3, Element(b'\xcc', 'l2'))
    assert m.load(0) == (Element(b'\x00' * 3 + b'\xcc\xaa\xbb' + b'\x00'*26, None), {'l1', 'l2'})

    m.store(3, Element(b'\xdd\xee', 'l3'))
    assert m.load(0) == (Element(b'\x00' * 3 + b'\xdd\xee\xbb' + b'\x00'*26, None), {'l1', 'l3'})


    m = Memory()
    m.store(3, Element(b'\xaa\xbb\xcc', 'l1'))
    m.store(4, Element(b'\xdd', 'l2'))
    assert m.load(0) == (Element(b'\x00' * 3 + b'\xaa\xdd\xcc' + b'\x00'*26, None), {'l1', 'l2'})
    m.store(2, Element(b'\xee\xff', 'l3'))
    assert m.load(0) == (Element(b'\x00' * 2 + b'\xee\xff\xdd\xcc' + b'\x00'*26, None), {'l1', 'l2', 'l3'})

    m.store(10, Element(b'\xaa' + b'\x00' * 30 + b'\xbb', 'l4'))
    assert m.load(10) == (Element(b'\xaa' + b'\x00' * 30 + b'\xbb', 'l4'), {'l4'})

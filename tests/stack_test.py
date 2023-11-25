from evmole.evm.stack import Stack

def test_stack():
    s = Stack()
    s.push(b'\xaa')
    assert s.pop() == b'\xaa'

    s.push(b'\xaa')
    s.push(b'\xbb')
    assert s.pop() == b'\xbb'
    assert s.pop() == b'\xaa'

    s.push(b'\xaa')
    assert s.peek() == b'\xaa'
    assert s.pop() == b'\xaa'
    assert s.peek() is None

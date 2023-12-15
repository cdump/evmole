import pytest

from evmole.evm.stack import Stack


def test_stack():
    s = Stack()
    s.push(b'\xaa' * 32)
    assert s.pop() == b'\xaa' * 32

    s.push(b'\xaa' * 32)
    s.push(b'\xbb' * 32)
    assert s.pop() == b'\xbb' * 32
    assert s.pop() == b'\xaa' * 32

    s.push(b'\xaa' * 32)
    assert s.peek() == b'\xaa' * 32
    assert s.pop() == b'\xaa' * 32
    with pytest.raises(IndexError):
        s.peek()

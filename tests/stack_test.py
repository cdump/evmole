import pytest

from evmole.evm.element import Element
from evmole.evm.stack import Stack, StackIndexError


def test_stack():
    s = Stack()
    s.push(Element(b'\xaa' * 32))
    assert s.pop().data == b'\xaa' * 32

    s.push(Element(b'\xaa' * 32))
    s.push(Element(b'\xbb' * 32))
    assert s.pop().data == b'\xbb' * 32
    assert s.pop().data == b'\xaa' * 32

    s.push(Element(b'\xaa' * 32))
    assert s.peek().data == b'\xaa' * 32
    assert s.pop().data == b'\xaa' * 32
    with pytest.raises(StackIndexError):
        s.peek()

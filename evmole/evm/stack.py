from .element import Element


class StackIndexError(Exception):
    pass


class Stack:
    def __init__(self):
        self._data: list[Element] = []

    def __str__(self):
        r = f'{len(self._data)} elems:'
        return r + ('\n' if len(self._data) else '') + '\n'.join(f'  - {el.data.hex()} | {el.label}' for el in self._data)

    def push(self, val: Element):
        assert len(val.data) == 32
        self._data.append(val)

    def pop(self) -> Element:
        try:
            return self._data.pop()
        except IndexError as e:
            raise StackIndexError from e

    def peek(self) -> Element:
        if len(self._data) == 0:
            raise StackIndexError
        return self._data[-1]

    def dup(self, n: int):
        if len(self._data) < n:
            raise StackIndexError
        self.push(self._data[-n])

    def swap(self, n: int):
        if len(self._data) <= n:
            raise StackIndexError
        self._data[-1], self._data[-n - 1] = self._data[-n - 1], self._data[-1]

    def push_uint(self, val: int):
        self.push(Element(data=val.to_bytes(32, byteorder='big', signed=False)))

    def pop_uint(self) -> int:
        return int.from_bytes(self.pop().data, 'big', signed=False)

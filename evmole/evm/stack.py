class Stack:
    def __init__(self):
        self._data = []

    def __str__(self):
        r = f'{len(self._data)} elems:'
        return r + ('\n' if len(self._data) else '') + '\n'.join(f'  - {el.hex()} | {type(el)}' for el in self._data)

    def push(self, val: bytes):
        assert len(val) == 32
        self._data.append(val)

    def pop(self) -> bytes:
        return self._data.pop()

    def peek(self, n: int = 0) -> bytes:
        return self._data[-1 * (n + 1)]

    def dup(self, n: int):
        self.push(self._data[-n])

    def swap(self, n: int):
        self._data[-1], self._data[-n - 1] = self._data[-n - 1], self._data[-1]

    def push_uint(self, val: int):
        self.push(val.to_bytes(32, byteorder='big', signed=False))

    def pop_uint(self) -> int:
        return int.from_bytes(self.pop(), 'big', signed=False)

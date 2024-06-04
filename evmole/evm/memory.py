from .element import Element


class Memory:
    def __init__(self):
        self._data: list[tuple[int, Element]] = []

    def __str__(self):
        r = f'{len(self._data)} elems:\n'
        return r + '\n'.join(f'  - {off}: {val.data.hex()} | {val.label}' for off, val in self._data)

    def store(self, offset: int, value: Element):
        self._data.append((offset, value))

    def size(self) -> int:
        if len(self._data) == 0:
            return 0
        return max(off + len(val.data) for off, val in self._data)

    def load(self, offset: int) -> tuple[bytes, set]:
        used = set()
        res = [b'\x00'] * 32
        for idx in range(32):
            i = idx + offset
            for off, val in reversed(self._data):
                if i >= off and i < off + len(val.data):
                    res[idx] = val.data[i - off : i - off + 1]
                    if val.label is not None:
                        used.add(val.label)
                    break

        ret = b''.join(res)
        return ret, used

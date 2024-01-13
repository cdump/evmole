class Memory:
    def __init__(self):
        self._data: list[tuple[int, bytes]] = []

    def __str__(self):
        r = f'{len(self._data)} elems:\n'
        return r + '\n'.join(f'  - {off}: {val.hex()} | {type(val).__name__}' for off, val in self._data)

    def store(self, offset: int, value: bytes):
        self._data.append((offset, value))

    def load(self, offset: int) -> tuple[bytes, set[bytes]]:
        used = set()
        res = [b'\x00'] * 32
        for idx in range(32):
            i = idx + offset
            for off, val in reversed(self._data):
                if i >= off and i < off + len(val):
                    res[idx] = val[i - off : i - off + 1]
                    used.add(val)
                    break

        ret = b''.join(res)
        return ret, used

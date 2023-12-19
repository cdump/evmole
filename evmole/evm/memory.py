class Memory:
    def __init__(self):
        self._seq = 0
        self._data: list[tuple[int, int, bytes]] = []

    def __str__(self):
        r = f'{len(self._data)} elems:\n'
        return r + '\n'.join(f'  - {off},{seq}: {val.hex()} | {type(val).__name__}' for off, seq, val in self._data)

    def store(self, offset: int, value: bytes):
        self._data.append((offset, self._seq, value))
        self._seq += 1

    def load(self, offset: int) -> tuple[bytes, set[bytes]]:
        res: list[tuple[int, bytes, bytes | None]] = [(0, b'\x00', None)] * 32
        for i in range(offset, offset + 32):
            idx = i - offset
            for off, seq, val in self._data:
                if seq >= res[idx][0] and i >= off and i < off + len(val):
                    res[idx] = (seq, val[i - off : i - off + 1], val)

        ret = b''.join(v[1] for v in res)
        used = set(v[2] for v in res if v[2] is not None)
        return ret, used

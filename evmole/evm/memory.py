class Memory:
    def __init__(self):
        self._data = []

    def __str__(self):
        r = f'{len(self._data)} elems:\n'
        return r + '\n'.join(f'  - {off}: {val.hex()} | {type(val)}' for off, val in self._data)

    def store(self, offset: int, value: bytes):
        self._data.append((offset, value))

    def load(self, offset: int) -> tuple[bytes, list[bytes]]:
        self._data = sorted(self._data)
        ret = b''
        used = []
        for off, val in self._data:
            b = off + len(val)
            if b <= offset:
                continue
            if offset + (32 - len(ret)) <= off:
                break

            if off > offset:
                ret += b'\x00' * (off - offset) + val
            elif off < offset:
                ret += val[offset - off :]
            else:
                ret += val
            used.append(val)
            offset += b

        if len(ret) > 32:
            ret = ret[:32]
        else:
            ret += b'\x00' * max(0, 32 - len(ret))
        return ret, used

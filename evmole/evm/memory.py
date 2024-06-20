from .element import Element


class Memory:
    def __init__(self) -> None:
        self.data: list[tuple[int, Element]] = []

    def __str__(self):
        r = f'{len(self.data)} elems:\n'
        return r + '\n'.join(f'  - {off}: {val.data.hex()} | {val.label}' for off, val in self.data)

    def store(self, offset: int, value: Element):
        self.data.append((offset, value))

    def size(self) -> int:
        if len(self.data) == 0:
            return 0
        return max(off + len(val.data) for off, val in self.data)

    def get(self, offset: int):
        for off, val in reversed(self.data):
            if off == offset:
                return val
        return None

    def load(self, offset: int) -> tuple[Element, set]:
        used = set()
        res = [b'\x00'] * 32
        for idx in range(32):
            i = idx + offset
            for off, val in reversed(self.data):
                if i >= off and i < off + len(val.data):
                    if val.label is not None:
                        used.add(val.label)
                    # early return if it's one full element
                    if idx == 0 and offset == off and len(val.data) == 32:
                        return val, used
                    res[idx] = val.data[i - off : i - off + 1]
                    break

        return Element(data=b''.join(res)), used

from typing import Any


class Element:
    __match_args__ = ('label',)

    data: bytes
    label: Any | None

    def __init__(self, data: bytes, label: Any | None = None):
        self.data = data
        self.label = label

    def __len__(self):
        return len(self.data)

    def __str__(self):
        return f'{self.data.hex()} | {self.label}'

    def load(self, offset: int, size: int = 32):
        val = self.data[offset : min(offset + size, len(self.data))].ljust(size, b'\x00')
        return Element(data=val, label=self.label)

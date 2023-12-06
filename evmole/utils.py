def to_bytes(v: str | bytes) -> bytes:
    if isinstance(v, str):
        return bytes.fromhex(v[2:] if v.startswith('0x') else v)
    assert isinstance(v, bytes), 'must be hex-string or bytes'
    return v

export default class Memory {
  constructor() {
    this._data = []
  }

  toString() {
    let r = `${this._data.length} elems:\n`
    for (const el of this._data) {
      r += `   - ${el.toString(16)} | ${typeof el}\n`
    }
    return r
  }

  store(offset, value) {
    this._data.push([offset, value])
  }

  load(offset) {
    this._data = this._data.sort((a, b) => a[0] - b[0])
    let ret = []
    let used = []

    for (const [off, val] of this._data) {
      const b = off + val.length
      if (b <= offset) {
        continue
      }
      if (offset + (32 - ret.length) <= off) {
        break
      }

      if (off > offset) {
        ret.push(...new Array(off - offset).fill(0))
        ret.push(...val)
      } else if (off < offset) {
        ret.push(...val.subarray(offset - off))
      } else {
        ret.push(...val)
      }

      used.push(val)
      offset += b
    }

    if (ret.length > 32) {
      ret = ret.slice(0, 32)
    } else {
      ret.push(...new Array(32 - ret.length).fill(0))
    }

    return [new Uint8Array(ret), used]
  }
}

export default class Memory {
  constructor() {
    this._data = []
  }

  toString() {
    let r = `${this._data.length} elems:\n`
    r += this._data.map(([off, val]) => `  - ${off}: ${val.toString()}`).join('\n')
    return r
  }

  store(offset, value) {
    this._data.push([offset, value])
  }

  size() {
    if (this._data.length === 0) {
      return 0
    }
    return Math.max(...this._data.map(([off, val]) => off + val.data.length))
  }

  load(offset) {
    const ret = new Uint8Array(32)
    const used = new Set()

    for (let idx = 0; idx < 32; idx++) {
      const i = idx + offset
      for (let d = this._data.length - 1; d >= 0; d--) {
        const [off, val] = this._data[d]
        if (i >= off && i < off + val.data.length) {
          ret[idx] = val.data[i - off]
          used.add(val.label)
          break
        }
      }
    }

    return [ret, used]
  }
}

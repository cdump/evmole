export default class Memory {
  constructor() {
    this._data = []
  }

  toString() {
    let r = `${this._data.length} elems:\n`
    r += this._data.map(([off,val]) => `  - ${off}: ${val.reduce((acc, v) => acc + v.toString(16).padStart(2, '0'), '')} | ${val.constructor.name}`).join('\n')
    return r
  }

  store(offset, value) {
    this._data.push([offset, value])
  }

  load(offset) {
    const ret = new Uint8Array(32)
    const used = new Set()

    for (let idx = 0; idx < 32; idx++) {
      const i = idx + offset
      for (let d = this._data.length - 1; d >= 0; d--) {
        const [off, val] = this._data[d]
        if (i >= off && i < off + val.length) {
          ret[idx] = val[i - off]
          used.add(val)
          break
        }
      }
    }

    return [ret, used]
  }
}

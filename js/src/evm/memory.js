import Element from './element.js'

export default class Memory {
  constructor() {
    this.data = []
  }

  toString() {
    let r = `${this.data.length} elems:\n`
    r += this.data.map(([off, val]) => `  - ${off}: ${val.toString()}`).join('\n')
    return r
  }

  store(offset, value) {
    this.data.push([offset, value])
  }

  size() {
    if (this.data.length === 0) {
      return 0
    }
    return Math.max(...this.data.map(([off, val]) => off + val.data.length))
  }

  get(offset) {
    for (let d = this.data.length - 1; d >= 0; d--) {
      const [off, val] = this.data[d]
      if (off == offset) {
        return val
      }
    }
    return undefined
  }

  load(offset) {
    const ret = new Uint8Array(32)
    const used = new Set()

    for (let idx = 0; idx < 32; idx++) {
      const i = idx + offset
      for (let d = this.data.length - 1; d >= 0; d--) {
        const [off, val] = this.data[d]
        if (i >= off && i < off + val.data.length) {
          // early return if it's one full element
          if (val.label !== undefined) {
            used.add(val.label)
          }
          if (idx === 0 && offset === off && val.data.length === 32) {
            return [val, used]
          }
          ret[idx] = val.data[i - off]
          break
        }
      }
    }

    return [new Element(ret), used];
  }
}

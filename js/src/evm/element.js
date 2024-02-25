export default class Element {
  constructor(data, label) {
    this.data = data
    this.label = label
  }

  get length() {
    return this.data.length
  }

  toString() {
    return `${this.data.reduce((acc, v) => acc + v.toString(16).padStart(2, '0'), '')} | ${this.label !== undefined ? this.label : 'None'}`
  }

  load(offset, size = 32) {
    const v = new Uint8Array(size)
    v.set(this.data.subarray(offset, offset + size))
    return new Element(v, this.label)
  }
}

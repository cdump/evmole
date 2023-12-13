import { bigIntToUint8Array, uint8ArrayToBigInt } from '../utils.js'

export default class Stack {
  constructor() {
    this._data = []
  }

  toString() {
    let r = `${this._data.length} elems:\n`
    r += this._data.map((el) => `  - ${el.reduce((acc, v) => acc + v.toString(16).padStart(2, '0'), '')} | ${typeof el}`).join('\n')
    return r
  }

  push(val) {
    this._data.push(val)
  }

  pop() {
    return this._data.pop()
  }

  peek(idx = 0) {
    return this._data[this._data.length - idx - 1]
  }

  dup(n) {
    this._data.push(this._data[this._data.length - n])
  }

  swap(n) {
    const tmp = this._data[this._data.length - n - 1]
    this._data[this._data.length - n - 1] = this._data[this._data.length - 1]
    this._data[this._data.length - 1] = tmp
  }

  push_uint(val) {
    this.push(bigIntToUint8Array(val))
  }

  pop_uint() {
    return uint8ArrayToBigInt(this.pop())
  }
}

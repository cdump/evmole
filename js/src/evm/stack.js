import { bigIntToUint8Array, uint8ArrayToBigInt } from '../utils.js'

export class StackIndexError extends Error {}

export default class Stack {
  constructor() {
    this._data = []
  }

  toString() {
    let r = `${this._data.length} elems:\n`
    r += this._data.map((el) => `  - ${el.reduce((acc, v) => acc + v.toString(16).padStart(2, '0'), '')} | ${el.constructor.name}`).join('\n')
    return r
  }

  push(val) {
    this._data.push(val)
  }

  pop() {
    const v = this._data.pop()
    if (v === undefined) {
      throw new StackIndexError()
    }
    return v
  }

  peek() {
    const v = this._data[this._data.length - 1]
    if (v === undefined) {
      throw new StackIndexError()
    }
    return v
  }

  dup(n) {
    const v = this._data[this._data.length - n]
    if (v === undefined) {
      throw new StackIndexError()
    }
    this._data.push(v)
  }

  swap(n) {
    if (this._data.length <= n) {
      throw new StackIndexError()
    }
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

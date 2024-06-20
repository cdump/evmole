import { bigIntToUint8Array, uint8ArrayToBigInt } from '../utils.js'
import Element from './element.js'

export class StackIndexError extends Error {}

export default class Stack {
  constructor() {
    this.data = []
  }

  toString() {
    let r = `${this.data.length} elems:\n`
    r += this.data.map((el) => `  - ${el.toString()}`).join('\n')
    return r
  }

  push(val) {
    this.data.push(val)
  }

  pop() {
    const v = this.data.pop()
    if (v === undefined) {
      throw new StackIndexError()
    }
    return v
  }

  peek() {
    const v = this.data[this.data.length - 1]
    if (v === undefined) {
      throw new StackIndexError()
    }
    return v
  }

  dup(n) {
    const v = this.data[this.data.length - n]
    if (v === undefined) {
      throw new StackIndexError()
    }
    this.data.push(v)
  }

  swap(n) {
    if (this.data.length <= n) {
      throw new StackIndexError()
    }
    const tmp = this.data[this.data.length - n - 1]
    this.data[this.data.length - n - 1] = this.data[this.data.length - 1]
    this.data[this.data.length - 1] = tmp
  }

  push_uint(val) {
    this.push(new Element(bigIntToUint8Array(val)))
  }

  pop_uint() {
    return uint8ArrayToBigInt(this.pop().data)
  }
}

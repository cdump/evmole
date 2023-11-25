export default class Stack {
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

  push(val) {
    this._data.push(val)
  }

  pop() {
    return this._data.pop()
  }

  peek(idx = 0) {
    if (this._data.length < idx + 1) {
      return undefined
    }
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
}

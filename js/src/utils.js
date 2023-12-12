export function hexToUint8Array(str) {
  let start = 0
  if (str.startsWith('0x')) {
    start = 2
  }
  const arr = new Uint8Array((str.length - start + 1) / 2)
  let p = 0
  for (let i = start; i < str.length; i += 2) {
    arr[p] = parseInt(str.slice(i, i + 2), 16)
    p++
  }
  return arr
}

export function uint8ArrayToBigInt(arr) {
    return arr.reduce((acc, b) => acc * 256n + BigInt(b), 0n)
}

export function bigIntToUint8Array(val, n=32) {
  const r = new Uint8Array(n);
  while(n > 0) {
    r[--n] = Number(val & 255n);
    val >>= 8n;
  }
  return r;
}

export function toBigInt(v) {
  if (typeof v.toBigInt === 'function') return v.toBigInt()
  if (!(v instanceof Uint8Array)) throw `Not uint8array instance`
  return uint8ArrayToBigInt(v)
}

export function modExp(a, b, n) {
  a = a % n
  var result = 1n
  var x = a
  while (b > 0) {
    var leastSignificantBit = b % 2n
    b = b / 2n
    if (leastSignificantBit == 1n) {
      result = result * x
      result = result % n
    }
    x = x * x
    x = x % n
  }
  return result
}

export function bigIntBitLength(v) {
  return v.toString(2).length
}

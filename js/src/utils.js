export function hexToUint8Array(input) {
  let str = input
  if (str.startsWith('0x')) {
    str = str.slice(2)
  }
  if (str.length % 2 !== 0) {
    str = `0${str}`
  }
  if (typeof Buffer !== 'undefined') {
    // fast path for nodejs:
    return Buffer.from(str, 'hex')
  }
  const arr = new Uint8Array(str.length / 2)
  for (let i = 0, p = 0; i < str.length; i += 2, p += 1) {
    arr[p] = Number.parseInt(str.slice(i, i + 2), 16)
  }
  return arr
}

export function uint8ArrayToBigInt(arr) {
  return arr.reduce((acc, b) => acc * 256n + BigInt(b), 0n)
}

export function bigIntToUint8Array(val, n = 32) {
  const result = new Uint8Array(n)
  let rv = val
  for (let i = n - 1; i >= 0; i--) {
    result[i] = Number(rv & 255n)
    rv >>= 8n
  }
  return result
}

export function toBigInt(v) {
  if (v instanceof Uint8Array) {
    return uint8ArrayToBigInt(v)
  }
  throw 'Not Uint8Array instance'
}

export function toUint8Array(v) {
  if (v instanceof Uint8Array) {
    return v
  }
  if (typeof v === 'string') {
    return hexToUint8Array(v)
  }
  throw 'Must be hex-string or Uint8Array'
}

export function modExp(base, exponent, modulus) {
  if (modulus === 1n) {
    return 0n
  }

  let result = 1n
  let x = base % modulus
  let e = exponent

  while (e > 0n) {
    if (e & 1n) {
      result = (result * x) % modulus
    }
    x = (x * x) % modulus
    e >>= 1n
  }

  return result
}

export function bigIntBitLength(v) {
  return v.toString(2).length
}

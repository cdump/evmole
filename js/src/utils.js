export function hexToUint8Array(str) {
  let start = 0
  if (str.startsWith('0x')) {
    start = 2
  }
  const arr = new Uint8Array((str.length - start) / 2)
  let p = 0
  for (let i = start; i < str.length; i += 2) {
    arr[p] = parseInt(str.slice(i, i + 2), 16)
    p++
  }
  return arr
}

export function uint8ArrayToBigInt(arr) {
  return BigInt(
    arr.reduce((acc, v) => acc + v.toString(16).padStart(2, '0'), '0x'),
  )
}

export function bigIntToUint8Array(val) {
  return hexToUint8Array(val.toString(16))
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

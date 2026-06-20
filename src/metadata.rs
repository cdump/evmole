//! Extraction of terminal CBOR metadata.

use minicbor::{Decoder, data::Type};

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CborMetadata {
    #[cfg_attr(feature = "serde", serde(rename = "bytecodeOffset"))]
    pub bytecode_offset: usize,
    #[cfg_attr(feature = "serde", serde(rename = "cborLength"))]
    pub cbor_length: usize,
    pub entries: Vec<CborEntry>,
}

/// A CBOR map entry whose key is a text string.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CborEntry {
    pub key: String,
    pub value: CborValue,
}

/// A commonly useful CBOR scalar, or the original encoding of any other value.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "type", content = "value", rename_all = "lowercase")
)]
pub enum CborValue {
    String(String),
    Integer(i64),
    Bytes(#[cfg_attr(feature = "javascript", serde(serialize_with = "serialize_hex"))] Vec<u8>),
    Bool(bool),
    Undecoded(#[cfg_attr(feature = "javascript", serde(serialize_with = "serialize_hex"))] Vec<u8>),
}

#[cfg(feature = "javascript")]
fn serialize_hex<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&alloy_primitives::hex::encode(bytes))
}

/// Extract a length-suffixed, terminal CBOR map from deployed bytecode.
pub(crate) fn extract(code: &[u8]) -> Option<CborMetadata> {
    if code.len() < 2 {
        return None;
    }
    let cbor_length = u16::from_be_bytes([code[code.len() - 2], code[code.len() - 1]]) as usize;
    if cbor_length == 0 || cbor_length > code.len() - 2 {
        return None;
    }
    let bytecode_offset = code.len() - 2 - cbor_length;
    let candidate = &code[bytecode_offset..code.len() - 2];

    let mut validation = Decoder::new(candidate);
    if !matches!(validation.datatype().ok()?, Type::Map | Type::MapIndef)
        || validation.skip().is_err()
        || validation.position() != candidate.len()
    {
        return None;
    }

    Some(CborMetadata {
        bytecode_offset,
        cbor_length,
        entries: decode_entries(candidate)?,
    })
}

fn decode_entries(candidate: &[u8]) -> Option<Vec<CborEntry>> {
    let mut d = Decoder::new(candidate);
    let len = d.map().ok()?;
    let mut remaining = len;
    let mut entries = Vec::new();

    loop {
        if let Some(n) = remaining {
            if n == 0 {
                break;
            }
            remaining = Some(n - 1);
        } else if d.datatype().ok()? == Type::Break {
            d.set_position(d.position() + 1);
            break;
        }

        let key = match d.datatype().ok()? {
            Type::String | Type::StringIndef => Some(decode_string(&mut d)?),
            _ => {
                d.skip().ok()?;
                None
            }
        };

        if let Some(key) = key {
            entries.push(CborEntry {
                key,
                value: decode_value(&mut d, candidate)?,
            });
        } else {
            d.skip().ok()?;
        }
    }
    Some(entries)
}

fn decode_string(d: &mut Decoder<'_>) -> Option<String> {
    let mut value = String::new();
    for part in d.str_iter().ok()? {
        value.push_str(part.ok()?);
    }
    Some(value)
}

fn decode_bytes(d: &mut Decoder<'_>) -> Option<Vec<u8>> {
    let mut value = Vec::new();
    for part in d.bytes_iter().ok()? {
        value.extend_from_slice(part.ok()?);
    }
    Some(value)
}

fn decode_value(d: &mut Decoder<'_>, candidate: &[u8]) -> Option<CborValue> {
    let start = d.position();
    let value = match d.datatype().ok()? {
        Type::String | Type::StringIndef => CborValue::String(decode_string(d)?),
        Type::Bytes | Type::BytesIndef => CborValue::Bytes(decode_bytes(d)?),
        Type::Bool => CborValue::Bool(d.bool().ok()?),
        Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::Int => {
            let integer = i128::from(d.int().ok()?);
            // Keep the public representation lossless in JavaScript as well as Rust,
            // Python, Go, and JSON. Larger CBOR integers retain their exact encoding.
            const MIN_SAFE_INTEGER: i128 = -9_007_199_254_740_991;
            const MAX_SAFE_INTEGER: i128 = 9_007_199_254_740_991;
            match i64::try_from(integer)
                .ok()
                .filter(|_| (MIN_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&integer))
            {
                Some(integer) => CborValue::Integer(integer),
                None => CborValue::Undecoded(candidate[start..d.position()].to_vec()),
            }
        }
        _ => {
            d.skip().ok()?;
            CborValue::Undecoded(candidate[start..d.position()].to_vec())
        }
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trailer(cbor: &[u8]) -> Vec<u8> {
        let mut code = vec![0x60, 0x00];
        code.extend(cbor);
        code.extend_from_slice(&(cbor.len() as u16).to_be_bytes());
        code
    }

    #[test]
    fn decodes_supported_values_and_preserves_duplicates() {
        let cbor = [
            0xa6, 0x61, b's', 0x62, b'h', b'i', 0x61, b'i', 0x20, 0x61, b'b', 0x42, 1, 2, 0x61,
            b'f', 0xf5, 0x61, b'x', 0x82, 1, 2, 0x61, b's', 0x61, b'!',
        ];
        let result = extract(&trailer(&cbor)).unwrap();
        assert_eq!(result.bytecode_offset, 2);
        assert_eq!(result.cbor_length, cbor.len());
        assert_eq!(
            result.entries,
            vec![
                CborEntry {
                    key: "s".into(),
                    value: CborValue::String("hi".into())
                },
                CborEntry {
                    key: "i".into(),
                    value: CborValue::Integer(-1)
                },
                CborEntry {
                    key: "b".into(),
                    value: CborValue::Bytes(vec![1, 2])
                },
                CborEntry {
                    key: "f".into(),
                    value: CborValue::Bool(true)
                },
                CborEntry {
                    key: "x".into(),
                    value: CborValue::Undecoded(vec![0x82, 1, 2])
                },
                CborEntry {
                    key: "s".into(),
                    value: CborValue::String("!".into())
                },
            ]
        );
    }

    #[test]
    fn skips_non_string_keys() {
        let cbor = [0xa2, 0x01, 0x61, b'x', 0x61, b'k', 0x02];
        assert_eq!(
            extract(&trailer(&cbor)).unwrap().entries,
            vec![CborEntry {
                key: "k".into(),
                value: CborValue::Integer(2)
            }]
        );
    }

    #[test]
    fn preserves_integers_outside_the_portable_range() {
        let cbor = [
            0xa1, 0x61, b'i', 0x1b, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(
            extract(&trailer(&cbor)).unwrap().entries[0].value,
            CborValue::Undecoded(vec![0x1b, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
        );
    }

    #[test]
    fn supports_indefinite_strings_and_bytes() {
        // {_ "key": (_ h'01', h'0203'), "text": (_ "a", "b")}
        let cbor = [
            0xbf, 0x63, b'k', b'e', b'y', 0x5f, 0x41, 1, 0x42, 2, 3, 0xff, 0x64, b't', b'e', b'x',
            b't', 0x7f, 0x61, b'a', 0x61, b'b', 0xff, 0xff,
        ];
        assert_eq!(
            extract(&trailer(&cbor)).unwrap().entries,
            vec![
                CborEntry {
                    key: "key".into(),
                    value: CborValue::Bytes(vec![1, 2, 3])
                },
                CborEntry {
                    key: "text".into(),
                    value: CborValue::String("ab".into())
                },
            ]
        );
    }

    #[test]
    fn rejects_bad_framing() {
        assert!(extract(&[0xa0, 0, 2]).is_none());
        assert!(extract(&trailer(&[0x81, 0x01])).is_none());
        assert!(extract(&trailer(&[0xa0, 0x00])).is_none());
    }

    #[cfg(feature = "javascript")]
    #[test]
    fn serializes_byte_values_as_hex_for_javascript() {
        assert_eq!(
            serde_json::to_value(CborValue::Bytes(vec![0, 8, 26])).unwrap(),
            serde_json::json!({ "type": "bytes", "value": "00081a" })
        );
        assert_eq!(
            serde_json::to_value(CborValue::Undecoded(vec![0x82, 1, 2])).unwrap(),
            serde_json::json!({ "type": "undecoded", "value": "820102" })
        );
    }
}

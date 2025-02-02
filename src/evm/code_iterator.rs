use super::op;
use alloy_primitives::hex;

#[derive(Debug)]
pub struct CodeOp<'a> {
    #[allow(dead_code)]
    pub op: op::OpCode,
    pub opi: &'static op::OpCodeInfo,
    pub arg: &'a [u8],
}

pub fn iterate_code(code: &[u8]) -> impl Iterator<Item = (usize, CodeOp)> {
    let mut pc = 0;
    let code_len = code.len();
    std::iter::from_fn(move || {
        if pc >= code_len {
            return None;
        }
        let op = code[pc];
        let opi = op::info(op);
        if pc + opi.size > code_len {
            return None;
        }
        let curpc = pc;
        pc += opi.size;
        Some((
            curpc,
            CodeOp {
                op,
                opi,
                arg: &code[curpc + 1..pc],
            },
        ))
    })
}

pub fn disassemble(code: &[u8]) -> Vec<(usize, String)> {
    iterate_code(code)
        .map(|(pc, op)| {
            (
                pc,
                if op.arg.is_empty() {
                    op.opi.name.to_string()
                } else {
                    format!("{} {}", op.opi.name, hex::encode(op.arg))
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_code_disassemble() {
        let result = disassemble(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_invalid_code_disassemble() {
        let result = disassemble(&[0xb0, 0xb1, 0x01]);
        assert_eq!(
            result,
            vec![
                (0, "?".to_string()),
                (1, "?".to_string()),
                (2, "ADD".to_string())
            ]
        );
    }
}

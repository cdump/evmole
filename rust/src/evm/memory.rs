use std::{collections::HashSet, fmt};

#[derive(Clone)]
struct LabeledVec<T>(Vec<u8>, Option<T>);

#[derive(Clone)]
pub struct Memory<T> {
    seq: u32,
    data: Vec<(u32, u32, LabeledVec<T>)>,
}

impl<T: fmt::Debug> fmt::Debug for Memory<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} elems:", self.data.len())?;
        for (off, seq, val) in &self.data {
            write!(
                f,
                "\n  - {},{}: {} | {:?}",
                off,
                seq,
                val.0
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(""),
                val.1
            )?;
        }
        Ok(())
    }
}

impl<T> Memory<T>
where
    T: fmt::Debug + Clone + Eq + std::hash::Hash,
{
    pub fn new() -> Memory<T> {
        Memory {
            seq: 0,
            data: Vec::new(),
        }
    }
    pub fn store(&mut self, offset: u32, value: Vec<u8>, label: Option<T>) {
        self.data.push((offset, self.seq, LabeledVec(value, label)));
        self.seq += 1
    }

    pub fn load(&mut self, offset: u32) -> ([u8; 32], HashSet<T>) {
        let mut r: [u8; 32] = [0; 32];

        let mut res: [(u32, &Option<T>); 32] = [(0, &None); 32];
        for i in offset..offset + 32 {
            let idx = (i - offset) as usize;
            for (off, seq, data) in &self.data {
                if *seq >= res[idx].0 && i >= *off && i < *off + data.0.len() as u32 {
                    res[idx] = (*seq, &data.1);
                    r[idx] = data.0[(i - off) as usize];
                }
            }
        }

        let used: HashSet<T> = res.iter().filter_map(|v| v.1.clone()).collect();

        (r, used)
    }
}

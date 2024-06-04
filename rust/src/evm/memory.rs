use std::{collections::HashSet, fmt};

#[derive(Clone)]
struct LabeledVec<T>(Vec<u8>, Option<T>);

#[derive(Clone)]
pub struct Memory<T> {
    data: Vec<(u32, LabeledVec<T>)>,
}

impl<T: fmt::Debug> fmt::Debug for Memory<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} elems:", self.data.len())?;
        for (off, val) in &self.data {
            write!(
                f,
                "\n  - {}: {} | {:?}",
                off,
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
        Memory { data: Vec::new() }
    }
    pub fn store(&mut self, offset: u32, value: Vec<u8>, label: Option<T>) {
        self.data.push((offset, LabeledVec(value, label)));
    }

    pub fn size(&self) -> usize {
        self.data
            .iter()
            .map(|(off, data)| *off as usize + data.0.len())
            .max()
            .unwrap_or(0)
    }

    pub fn load(&mut self, offset: u32) -> ([u8; 32], HashSet<T>) {
        let mut r: [u8; 32] = [0; 32];
        let mut used: HashSet<T> = HashSet::new();

        #[allow(clippy::needless_range_loop)]
        for idx in 0usize..32 {
            let i = idx as u32 + offset;
            for (off, data) in self.data.iter().rev() {
                if i >= *off && i < *off + data.0.len() as u32 {
                    r[idx] = data.0[(i - off) as usize];
                    if let Some(label) = &data.1 {
                        used.insert(label.clone());
                    }
                    break;
                }
            }
        }

        (r, used)
    }
}

use super::element::Element;
use std::fmt;

#[derive(Clone)]
pub struct LabeledVec<T> {
    pub data: Vec<u8>,
    pub label: Option<T>,
}

#[derive(Clone)]
pub struct Memory<T> {
    pub data: Vec<(u32, LabeledVec<T>)>,
}

impl<T: fmt::Debug> fmt::Debug for Memory<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} elems:", self.data.len())?;
        for (off, val) in &self.data {
            write!(
                f,
                "\n  - {}: {} | {:?}",
                off,
                val.data
                    .iter()
                    .map(|x| format!("{:02x}", x))
                    .collect::<Vec<_>>()
                    .join(""),
                val.label
            )?;
        }
        Ok(())
    }
}

impl<T> Memory<T>
where
    T: Clone + PartialEq,
{
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }
    pub fn store(&mut self, offset: u32, data: Vec<u8>, label: Option<T>) {
        self.data.push((offset, LabeledVec { data, label }));
    }

    pub fn size(&self) -> usize {
        self.data
            .iter()
            .map(|(off, el)| *off as usize + el.data.len())
            .max()
            .unwrap_or(0)
    }

    pub fn get_mut(&mut self, offset: u32) -> Option<&mut LabeledVec<T>> {
        if let Some(el) = self.data.iter_mut().rev().find(|v| v.0 == offset) {
            return Some(&mut el.1);
        }
        None
    }

    pub fn load(&self, offset: u32, size: u32) -> (Vec<u8>, Vec<T>) {
        let mut data = vec![0; size as usize];
        let mut used = vec![];

        #[allow(clippy::needless_range_loop)]
        for idx in 0usize..size as usize {
            let i = idx + offset as usize;
            for (off, el) in self.data.iter().rev() {
                let uoff = *off as usize;
                if i >= uoff && i < uoff + el.data.len() {
                    if let Some(label) = &el.label {
                        if used.last() != Some(label) {
                            used.push(label.clone());
                        }
                    }
                    data[idx] = el.data[i - uoff];
                    break;
                }
            }
        }
        (data, used)
    }

    pub fn load_element(&self, offset: u32) -> (Element<T>, Vec<T>) {
        let mut r = Element {
            data: [0; 32],
            label: None,
        };
        let mut used: Vec<T> = Vec::new();

        #[allow(clippy::needless_range_loop)]
        for idx in 0usize..32 {
            let i = idx + offset as usize;
            for (off, el) in self.data.iter().rev() {
                let uoff = *off as usize;
                if i >= uoff && i < uoff + el.data.len() {
                    if let Some(label) = &el.label {
                        if used.last() != Some(label) {
                            used.push(label.clone());
                        }
                    }
                    r.data[idx] = el.data[i - uoff];
                    break;
                }
            }
        }
        (r, used)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_memory_is_empty() {
        let mem: Memory<String> = Memory::new();
        assert!(mem.data.is_empty());
        assert_eq!(mem.size(), 0);
    }

    #[test]
    fn test_store_and_load_simple() {
        let mut mem: Memory<String> = Memory::new();
        let data_to_store = vec![1, 2, 3, 4];
        let label = "label1".to_string();
        mem.store(10, data_to_store.clone(), Some(label.clone()));

        let expected_labels = vec![label];
        let (loaded_data, labels) = mem.load(10, 4);
        assert_eq!(loaded_data, data_to_store);
        assert_eq!(labels, expected_labels);

        // Test loading partial data
        let (loaded_partial, labels_partial) = mem.load(11, 2);
        assert_eq!(loaded_partial, vec![2, 3]);
        assert_eq!(labels_partial, expected_labels);

        // Test loading with zero padding
        let (loaded_padded, labels_padded) = mem.load(8, 8);
        assert_eq!(loaded_padded, vec![0, 0, 1, 2, 3, 4, 0, 0]);
        assert_eq!(labels_padded, expected_labels);
    }

    #[test]
    fn test_store_overwrite() {
        let mut mem: Memory<u32> = Memory::new();
        mem.store(0, vec![1; 5], Some(1)); // Store 5 bytes at offset 0
        mem.store(2, vec![2; 5], Some(2)); // Store 5 bytes at offset 2 (overlaps)
        assert_eq!(mem.size(), 7);

        let (loaded_data, labels) = mem.load(0, 7);
        assert_eq!(loaded_data, vec![1, 1, 2, 2, 2, 2, 2]);
        assert_eq!(labels, vec![1, 2]);
    }

    #[test]
    fn test_store_non_overlapping() {
        let mut mem: Memory<char> = Memory::new();
        mem.store(0, vec![10; 2], Some('a'));
        mem.store(5, vec![20; 3], Some('b'));
        assert_eq!(mem.size(), 8);

        let (loaded_data, labels) = mem.load(0, 8);
        assert_eq!(loaded_data, vec![10, 10, 0, 0, 0, 20, 20, 20]);
        assert_eq!(labels, vec!['a', 'b']);
    }

    #[test]
    fn test_load_element_simple() {
        let mut mem: Memory<String> = Memory::new();
        let mut data_to_store = vec![0; 32];
        data_to_store[0] = 1;
        data_to_store[31] = 255;
        mem.store(0, data_to_store.clone(), Some("word1".to_string()));

        let (element, labels) = mem.load_element(0);
        let expected_element_data: [u8; 32] = data_to_store.try_into().unwrap();
        assert_eq!(element.data, expected_element_data);
        assert_eq!(element.label, None);
        assert_eq!(labels, vec!["word1".to_string()]);
    }

    #[test]
    fn test_load_element_overwrite() {
        let mut mem: Memory<u8> = Memory::new();
        mem.store(0, vec![1; 32], Some(1));
        mem.store(16, vec![2; 32], Some(2));

        let (element, labels) = mem.load_element(0);

        let expected_data = [
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2,
        ];
        assert_eq!(element.data, expected_data);
        assert_eq!(element.label, None);
        assert_eq!(labels, vec![1, 2]);
    }

    #[test]
    fn test_load_element_partial_writes() {
        let mut mem: Memory<u8> = Memory::new();
        mem.store(0, vec![1; 10], Some(1)); // [0..10)
        mem.store(5, vec![2; 10], Some(2)); // Overlaps [5..15)
        mem.store(20, vec![3; 10], Some(3)); // Separate [20..30)

        let (element, labels) = mem.load_element(0); // Load word at 0 [0..32)

        let expected_data = [
            1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 0, 0,
        ];

        assert_eq!(element.data, expected_data);
        assert_eq!(element.label, None);
        assert_eq!(labels, vec![1, 2, 3]);
    }

    #[test]
    fn test_size() {
        let mut mem: Memory<bool> = Memory::new();
        assert_eq!(mem.size(), 0);

        mem.store(10, vec![0; 5], None); // [10..15) -> size 15
        assert_eq!(mem.size(), 15);

        mem.store(0, vec![0; 5], None); // [0..5) -> size still 15
        assert_eq!(mem.size(), 15);

        mem.store(20, vec![0; 10], None); // [20..30) -> size 30
        assert_eq!(mem.size(), 30);

        mem.store(25, vec![0; 10], None); // [25..35) -> size 35
        assert_eq!(mem.size(), 35);
    }
}

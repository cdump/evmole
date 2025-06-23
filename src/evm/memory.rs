use super::element::Element;
use std::{fmt, ops::Range};

#[derive(Clone)]
pub struct LabeledVec<T> {
    pub data: Vec<u8>,
    pub label: Option<T>,
}

#[derive(Clone)]
pub struct Memory<T> {
    pub data: Vec<(u32, LabeledVec<T>)>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct MemoryChunk<T> {
    pub dst_range: Range<usize>,
    pub src_range: Range<usize>,
    pub src_label: T,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct MemoryChunks<T> {
    pub chunks: Vec<MemoryChunk<T>>,

    // Total size of loaded memory, can be not equal to the sum of chunk sizes if some parts are loaded from zero memory.
    pub size: u32,
}

impl<T: Clone> MemoryChunks<T> {
    /// Creates a new `MemoryChunks` representing a slice of the current memory layout.
    ///
    /// This is analogous to a "load" operation on the memory layout itself,
    /// producing a new layout for a sub-region.
    pub fn load(&self, offset: u32, size: u32) -> MemoryChunks<T> {
        let offset = offset as usize;
        let load_end = offset + size as usize;

        let chunks = self
            .chunks
            .iter()
            .filter_map(|chunk| {
                // Find the intersection between the existing chunk's destination range
                // and the requested load range.
                let intersection_start = chunk.dst_range.start.max(offset);
                let intersection_end = chunk.dst_range.end.min(load_end);

                if intersection_start < intersection_end {
                    let intersection_len = intersection_end - intersection_start;
                    let new_dst_start = intersection_start - offset;
                    let offset_in_chunk = intersection_start - chunk.dst_range.start;
                    let new_src_start = chunk.src_range.start + offset_in_chunk;

                    Some(MemoryChunk {
                        dst_range: new_dst_start..(new_dst_start + intersection_len),
                        src_range: new_src_start..(new_src_start + intersection_len),
                        src_label: chunk.src_label.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        MemoryChunks { chunks, size }
    }
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

    fn load_to(&self, offset: u32, size: u32, data: &mut [u8]) -> MemoryChunks<T> {
        let mut chunks: Vec<MemoryChunk<T>> = Vec::new();
        #[allow(clippy::needless_range_loop)]
        for idx in 0..size as usize {
            let i = idx + offset as usize;
            for (off, el) in self.data.iter().rev() {
                let uoff = *off as usize;
                if i >= uoff && i < uoff + el.data.len() {
                    let rel_pos = i - uoff;
                    if let Some(label) = &el.label {
                        let new_chunk = MemoryChunk {
                            dst_range: idx..(idx + 1),
                            src_range: rel_pos..(rel_pos + 1),
                            src_label: label.clone(),
                        };

                        if let Some(last) = chunks.last_mut() {
                            if last.src_label == *label
                                && last.src_range.end == new_chunk.src_range.start
                                && last.dst_range.end == new_chunk.dst_range.start
                            {
                                last.src_range.end += 1;
                                last.dst_range.end += 1;
                            } else {
                                chunks.push(new_chunk);
                            }
                        } else {
                            chunks.push(new_chunk);
                        }
                    }
                    data[idx] = el.data[rel_pos];
                    break;
                }
            }
        }
        MemoryChunks { chunks, size }
    }

    pub fn load(&self, offset: u32, size: u32) -> (Vec<u8>, MemoryChunks<T>) {
        let mut data = vec![0; size as usize];
        let chunks = self.load_to(offset, size, &mut data);
        (data, chunks)
    }
    pub fn load_element(&self, offset: u32) -> (Element<T>, MemoryChunks<T>) {
        let mut data = [0; 32];
        let chunks = self.load_to(offset, 32, &mut data);
        (Element { data, label: None }, chunks)
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

        let (loaded_data, labels) = mem.load(10, 4);
        assert_eq!(loaded_data, data_to_store);
        assert_eq!(
            labels.chunks,
            vec![MemoryChunk {
                dst_range: 0..4,
                src_range: 0..4,
                src_label: label.clone(),
            }]
        );

        // Test loading partial data
        let (loaded_partial, labels_partial) = mem.load(11, 2);
        assert_eq!(loaded_partial, vec![2, 3]);
        assert_eq!(
            labels_partial,
            MemoryChunks {
                chunks: vec![MemoryChunk {
                    dst_range: 0..2,
                    src_range: 1..3,
                    src_label: label.clone(),
                }],
                size: 2,
            }
        );

        // Test loading with zero padding
        let (loaded_padded, labels_padded) = mem.load(8, 8);
        assert_eq!(loaded_padded, vec![0, 0, 1, 2, 3, 4, 0, 0]);
        assert_eq!(
            labels_padded,
            MemoryChunks {
                chunks: vec![
                    // 0..2 are zero-padded
                    MemoryChunk {
                        dst_range: 2..6,
                        src_range: 0..4,
                        src_label: label.clone(),
                    },
                    // 6..8 are zero-padded
                ],
                size: 8,
            }
        );
    }

    #[test]
    fn test_store_overwrite() {
        let mut mem: Memory<u32> = Memory::new();
        mem.store(0, vec![1; 5], Some(1)); // Store 5 bytes at offset 0
        mem.store(2, vec![2; 5], Some(2)); // Store 5 bytes at offset 2 (overlaps)
        assert_eq!(mem.size(), 7);

        let (loaded_data, labels) = mem.load(0, 7);
        assert_eq!(loaded_data, vec![1, 1, 2, 2, 2, 2, 2]);
        assert_eq!(
            labels,
            MemoryChunks {
                chunks: vec![
                    MemoryChunk {
                        dst_range: 0..2,
                        src_range: 0..2,
                        src_label: 1,
                    },
                    MemoryChunk {
                        dst_range: 2..7,
                        src_range: 0..5,
                        src_label: 2,
                    }
                ],
                size: 7,
            }
        );
    }

    #[test]
    fn test_store_non_overlapping() {
        let mut mem: Memory<char> = Memory::new();
        mem.store(0, vec![10; 2], Some('a'));
        mem.store(5, vec![20; 3], Some('b'));
        assert_eq!(mem.size(), 8);

        let (loaded_data, labels) = mem.load(0, 8);
        assert_eq!(loaded_data, vec![10, 10, 0, 0, 0, 20, 20, 20]);
        assert_eq!(
            labels,
            MemoryChunks {
                chunks: vec![
                    MemoryChunk {
                        dst_range: 0..2,
                        src_range: 0..2,
                        src_label: 'a',
                    },
                    // 2..5 are zero-padded
                    MemoryChunk {
                        dst_range: 5..8,
                        src_range: 0..3,
                        src_label: 'b',
                    }
                ],
                size: 8,
            }
        );
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
        assert_eq!(
            labels,
            MemoryChunks {
                chunks: vec![MemoryChunk {
                    dst_range: 0..32,
                    src_range: 0..32,
                    src_label: "word1".to_string(),
                }],
                size: 32,
            }
        );
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
        assert_eq!(
            labels,
            MemoryChunks {
                chunks: vec![
                    MemoryChunk {
                        dst_range: 0..16,
                        src_range: 0..16,
                        src_label: 1,
                    },
                    MemoryChunk {
                        dst_range: 16..32,
                        src_range: 0..16,
                        src_label: 2,
                    }
                ],
                size: 32,
            }
        );
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
        assert_eq!(
            labels,
            MemoryChunks {
                chunks: vec![
                    MemoryChunk {
                        dst_range: 0..5,
                        src_range: 0..5,
                        src_label: 1,
                    },
                    MemoryChunk {
                        dst_range: 5..15,
                        src_range: 0..10,
                        src_label: 2,
                    },
                    // 15..20 are zero-padded
                    MemoryChunk {
                        dst_range: 20..30,
                        src_range: 0..10,
                        src_label: 3,
                    }
                ],
                size: 32,
            }
        );
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

    #[test]
    fn test_load_from_memory_chunks() {
        let mut mem: Memory<String> = Memory::new();
        mem.store(0, vec![1, 2, 3, 4], Some("data1".to_string()));
        mem.store(10, vec![5, 6, 7, 8], Some("data2".to_string()));

        let (loaded_data, labels) = mem.load(0, 14);
        assert_eq!(loaded_data, vec![1, 2, 3, 4, 0, 0, 0, 0, 0, 0, 5, 6, 7, 8]);
        assert_eq!(
            labels,
            MemoryChunks {
                chunks: vec![
                    MemoryChunk {
                        dst_range: 0..4,
                        src_range: 0..4,
                        src_label: "data1".to_string(),
                    },
                    // Zero-padded from [4..10)
                    MemoryChunk {
                        dst_range: 10..14,
                        src_range: 0..4,
                        src_label: "data2".to_string(),
                    }
                ],
                size: 14,
            }
        );

        let labels_loaded = labels.load(10, 2);
        assert_eq!(
            labels_loaded,
            MemoryChunks {
                chunks: vec![MemoryChunk {
                    dst_range: 0..2,
                    src_range: 0..2,
                    src_label: "data2".to_string(),
                }],
                size: 2,
            }
        );

        let labels_loaded = labels.load(3, 8);
        assert_eq!(
            labels_loaded,
            MemoryChunks {
                chunks: vec![
                    MemoryChunk {
                        dst_range: 0..1,
                        src_range: 3..4,
                        src_label: "data1".to_string(),
                    },
                    MemoryChunk {
                        dst_range: 7..8,
                        src_range: 0..1,
                        src_label: "data2".to_string(),
                    }
                ],
                size: 8,
            }
        );

        let labels_loaded = labels.load(10, 4);
        assert_eq!(
            labels_loaded,
            MemoryChunks {
                chunks: vec![MemoryChunk {
                    dst_range: 0..4,
                    src_range: 0..4,
                    src_label: "data2".to_string(),
                }],
                size: 4,
            }
        );
    }

    #[test]
    fn test_load_from_memory_chunks_second() {
        let mut mem: Memory<String> = Memory::new();
        mem.store(0, vec![1, 2, 3, 4], Some("data1".to_string()));
        mem.store(4, vec![5, 6, 7, 8], Some("data2".to_string()));

        let (loaded_data, labels) = mem.load(0, 8);
        assert_eq!(loaded_data, vec![1, 2, 3, 4, 5, 6, 7, 8]);

        let labels_loaded = labels.load(4, 4);
        assert_eq!(
            labels_loaded,
            MemoryChunks {
                chunks: vec![MemoryChunk {
                    dst_range: 0..4,
                    src_range: 0..4,
                    src_label: "data2".to_string(),
                }],
                size: 4,
            }
        );
    }
}

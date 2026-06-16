//! In-memory representation of a merged firmware image.

use std::collections::BTreeMap;
use std::path::Path;

use crate::MergehexError;

/// Policy for handling overlapping bytes when merging inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlapPolicy {
    /// Abort with an error when an overlap is detected.
    #[default]
    Error,
    /// Replace existing bytes with bytes from the new input.
    Replace,
    /// Keep existing bytes and ignore bytes from the new input.
    Ignore,
}

impl std::str::FromStr for OverlapPolicy {
    type Err = MergehexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(OverlapPolicy::Error),
            "replace" => Ok(OverlapPolicy::Replace),
            "ignore" => Ok(OverlapPolicy::Ignore),
            _ => Err(MergehexError::InvalidArgument {
                detail: format!(
                    "invalid overlap policy '{}'; expected 'error', 'replace', or 'ignore'",
                    s
                ),
            }),
        }
    }
}

/// A sparse memory image keyed by absolute address.
#[derive(Debug, Clone, Default)]
pub struct MemoryMap {
    data: BTreeMap<u64, u8>,
}

impl MemoryMap {
    /// Create an empty memory map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a single byte at `address` according to the overlap policy.
    /// `input` is used only for error reporting.
    pub fn insert(
        &mut self,
        address: u64,
        byte: u8,
        policy: OverlapPolicy,
        input: &Path,
    ) -> Result<(), MergehexError> {
        match self.data.entry(address) {
            std::collections::btree_map::Entry::Vacant(e) => {
                e.insert(byte);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(_) => match policy {
                OverlapPolicy::Error => Err(MergehexError::Overlap {
                    address,
                    input: input.into(),
                }),
                OverlapPolicy::Replace => {
                    self.data.insert(address, byte);
                    Ok(())
                }
                OverlapPolicy::Ignore => Ok(()),
            },
        }
    }

    /// Set a byte unconditionally (useful for tests and direct construction).
    pub fn set(&mut self, address: u64, byte: u8) -> Result<(), MergehexError> {
        self.data.insert(address, byte);
        Ok(())
    }

    /// Merge a slice of `(address, byte)` pairs into this map.
    pub fn merge_bytes(
        &mut self,
        pairs: &[(u64, u8)],
        policy: OverlapPolicy,
        input: &Path,
    ) -> Result<(), MergehexError> {
        for &(addr, byte) in pairs {
            self.insert(addr, byte, policy, input)?;
        }
        Ok(())
    }

    /// Iterate over sorted `(address, byte)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (u64, u8)> + use<'_> {
        self.data.iter().map(|(&a, &b)| (a, b))
    }

    /// Return the number of bytes stored.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Split the memory map into contiguous chunks of `(start_address, bytes)`.
    pub fn chunks(&self) -> Vec<(u64, Vec<u8>)> {
        let mut chunks = Vec::new();
        let mut current_start: Option<u64> = None;
        let mut current: Vec<u8> = Vec::new();

        for (addr, byte) in self.iter() {
            match current_start {
                Some(start) if addr == start + current.len() as u64 => {
                    current.push(byte);
                }
                _ => {
                    if let Some(start) = current_start {
                        chunks.push((start, current));
                    }
                    current_start = Some(addr);
                    current = vec![byte];
                }
            }
        }

        if let Some(start) = current_start {
            chunks.push((start, current));
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn memory_map_sorted_iteration() {
        let mut map = MemoryMap::new();
        map.set(0x10, 1).unwrap();
        map.set(0x00, 2).unwrap();
        map.set(0x20, 3).unwrap();

        let pairs: Vec<_> = map.iter().collect();
        assert_eq!(pairs, vec![(0x00, 2), (0x10, 1), (0x20, 3)]);
    }

    #[test]
    fn overlap_error() {
        let mut map = MemoryMap::new();
        map.merge_bytes(&[(0x00, 1)], OverlapPolicy::Error, Path::new("a"))
            .unwrap();
        let err = map
            .merge_bytes(&[(0x00, 2)], OverlapPolicy::Error, Path::new("b"))
            .unwrap_err();
        assert!(matches!(err, MergehexError::Overlap { address: 0, .. }));
    }

    #[test]
    fn overlap_replace() {
        let mut map = MemoryMap::new();
        map.merge_bytes(&[(0x00, 1)], OverlapPolicy::Error, Path::new("a"))
            .unwrap();
        map.merge_bytes(&[(0x00, 2)], OverlapPolicy::Replace, Path::new("b"))
            .unwrap();
        assert_eq!(map.iter().next().unwrap().1, 2);
    }

    #[test]
    fn chunks_detects_gaps() {
        let mut map = MemoryMap::new();
        map.set(0x00, 1).unwrap();
        map.set(0x01, 2).unwrap();
        map.set(0x03, 3).unwrap();
        let chunks = map.chunks();
        assert_eq!(chunks, vec![(0x00, vec![1, 2]), (0x03, vec![3])]);
    }
}

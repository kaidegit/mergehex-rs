//! ELF file parsing for extractable loadable segments.

use std::path::Path;

use object::{File, Object, ObjectSegment, SegmentFlags};

use crate::MergehexError;

const PF_R: u32 = 0x4;
const PF_X: u32 = 0x1;
const PF_W: u32 = 0x2;

/// Extract `(address, byte)` pairs from loadable/readable ELF segments.
pub fn parse_elf_file(path: &Path) -> Result<Vec<(u64, u8)>, MergehexError> {
    let data = std::fs::read(path).map_err(|e| MergehexError::Io {
        path: path.into(),
        source: e,
    })?;

    let file = File::parse(&data[..]).map_err(|e| MergehexError::ElfParse {
        detail: e.to_string(),
    })?;

    let mut pairs = Vec::new();

    for segment in file.segments() {
        let is_useful = match segment.flags() {
            SegmentFlags::Elf { p_flags } => (p_flags & (PF_R | PF_X | PF_W)) != 0,
            // For other formats, be permissive and inspect data instead.
            _ => true,
        };

        if !is_useful {
            continue;
        }

        let address = segment.address();
        let bytes = segment.data().map_err(|e| MergehexError::ElfParse {
            detail: format!("failed to read segment data: {}", e),
        })?;

        if bytes.is_empty() {
            continue;
        }

        for (i, byte) in bytes.iter().enumerate() {
            pairs.push((address + i as u64, *byte));
        }
    }

    if pairs.is_empty() {
        return Err(MergehexError::ElfParse {
            detail: "no loadable segments found".into(),
        });
    }

    Ok(pairs)
}

//! Input file format detection and parsing.

use std::ffi::OsStr;
use std::path::PathBuf;

use crate::MergehexError;
use crate::elf::parse_elf_file;
use crate::hex::parse_hex_file;
use crate::memory::{MemoryMap, OverlapPolicy};

/// Supported input formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Hex,
    Bin,
    Elf,
}

impl InputFormat {
    /// Infer format from a file extension, if possible.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "hex" | "ihex" => Some(InputFormat::Hex),
            "bin" => Some(InputFormat::Bin),
            "elf" | "axf" | "o" | "out" => Some(InputFormat::Elf),
            _ => None,
        }
    }
}

/// A single input specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSpec {
    pub path: PathBuf,
    pub format: InputFormat,
    /// Offset applied to binary inputs only.
    pub bin_offset: u64,
}

impl InputSpec {
    /// Parse a CLI input string of the form `path[@offset]`.
    /// If `explicit_format` is provided, it overrides extension-based detection.
    pub fn parse(s: &str, explicit_format: Option<InputFormat>) -> Result<Self, MergehexError> {
        let (path_part, offset_part) = match s.split_once('@') {
            Some((p, o)) => (p, Some(o)),
            None => (s, None),
        };

        let path = PathBuf::from(path_part);
        let format = explicit_format
            .or_else(|| {
                path.extension()
                    .and_then(OsStr::to_str)
                    .and_then(InputFormat::from_extension)
            })
            .ok_or_else(|| MergehexError::UnsupportedFormat {
                path: path.clone(),
                detail: "unable to detect file format from extension; use --format".into(),
            })?;

        let bin_offset = match offset_part {
            Some(o) => {
                if format != InputFormat::Bin {
                    return Err(MergehexError::InvalidArgument {
                        detail: format!(
                            "offset is only supported for binary inputs: {}",
                            path.display()
                        ),
                    });
                }
                parse_offset(o)?
            }
            None => 0,
        };

        Ok(InputSpec {
            path,
            format,
            bin_offset,
        })
    }

    /// Load this input into `(address, byte)` pairs.
    pub fn load(&self) -> Result<Vec<(u64, u8)>, MergehexError> {
        match self.format {
            InputFormat::Hex => parse_hex_file(&self.path),
            InputFormat::Bin => {
                let bytes = std::fs::read(&self.path).map_err(|e| MergehexError::Io {
                    path: self.path.clone(),
                    source: e,
                })?;
                Ok(bytes
                    .into_iter()
                    .enumerate()
                    .map(|(i, b)| (self.bin_offset + i as u64, b))
                    .collect())
            }
            InputFormat::Elf => parse_elf_file(&self.path),
        }
    }

    /// Merge this input into the provided memory map.
    pub fn merge_into(
        &self,
        memory: &mut MemoryMap,
        policy: OverlapPolicy,
    ) -> Result<(), MergehexError> {
        let pairs = self.load()?;
        memory.merge_bytes(&pairs, policy, &self.path)
    }
}

/// Parse an address offset string (decimal, `0x` hex, or `0` octal).
fn parse_offset(s: &str) -> Result<u64, MergehexError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(MergehexError::InvalidArgument {
            detail: "empty offset".into(),
        });
    }
    let value = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16)
    } else {
        s.parse::<u64>()
    }
    .map_err(|e| MergehexError::InvalidArgument {
        detail: format!("invalid offset '{}': {}", s, e),
    })?;
    Ok(value)
}

/// Helper to convert CLI format strings into `InputFormat`.
pub fn parse_format(s: &str) -> Result<InputFormat, MergehexError> {
    match s.to_lowercase().as_str() {
        "hex" | "ihex" => Ok(InputFormat::Hex),
        "bin" | "binary" => Ok(InputFormat::Bin),
        "elf" => Ok(InputFormat::Elf),
        _ => Err(MergehexError::InvalidArgument {
            detail: format!("invalid format '{}'; expected 'hex', 'bin', or 'elf'", s),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bin_with_offset() {
        let spec = InputSpec::parse("fw.bin@0x1000", None).unwrap();
        assert_eq!(spec.path, PathBuf::from("fw.bin"));
        assert_eq!(spec.format, InputFormat::Bin);
        assert_eq!(spec.bin_offset, 0x1000);
    }

    #[test]
    fn parse_hex_without_offset() {
        let spec = InputSpec::parse("fw.hex", None).unwrap();
        assert_eq!(spec.path, PathBuf::from("fw.hex"));
        assert_eq!(spec.format, InputFormat::Hex);
        assert_eq!(spec.bin_offset, 0);
    }

    #[test]
    fn offset_on_hex_is_rejected() {
        let err = InputSpec::parse("fw.hex@0x100", None).unwrap_err();
        assert!(matches!(err, MergehexError::InvalidArgument { .. }));
    }
}

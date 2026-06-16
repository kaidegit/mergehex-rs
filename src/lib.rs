//! mergehex-rs: merge Intel HEX, binary, and ELF files into a single Intel HEX file.

use std::path::PathBuf;

pub mod elf;
pub mod hex;
pub mod input;
pub mod memory;

/// Errors that can occur while merging files.
#[derive(Debug, thiserror::Error)]
pub enum MergehexError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse Intel HEX file {path} at line {line}")]
    HexLine {
        path: PathBuf,
        line: usize,
        #[source]
        source: Box<MergehexError>,
    },

    #[error("Intel HEX parse error: {detail}")]
    HexParse { detail: String },

    #[error("Intel HEX checksum mismatch: expected {expected:02X}, computed {computed:02X}")]
    HexChecksum { expected: u8, computed: u8 },

    #[error("overlapping data detected at address 0x{address:08X} (input: {input})")]
    Overlap { address: u64, input: PathBuf },

    #[error("ELF parse error: {detail}")]
    ElfParse { detail: String },

    #[error("unsupported input format for {path}: {detail}")]
    UnsupportedFormat { path: PathBuf, detail: String },

    #[error("invalid argument: {detail}")]
    InvalidArgument { detail: String },
}

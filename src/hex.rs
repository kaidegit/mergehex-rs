//! Intel HEX (IHEX) parser and writer.

use std::fmt::Write as _;
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::MergehexError;
use crate::memory::MemoryMap;

/// Maximum number of data bytes in a single Intel HEX record.
const MAX_RECORD_LEN: usize = 32;

/// A parsed Intel HEX record.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Record {
    /// Data record: address offset + payload bytes.
    Data { offset: u16, data: Vec<u8> },
    /// End-of-file record.
    EndOfFile,
    /// Extended segment address record (type 02): segment base << 4.
    ExtendedSegmentAddress(u16),
    /// Start segment address record (type 03): CS:IP.
    StartSegmentAddress { cs: u16, ip: u16 },
    /// Extended linear address record (type 04): upper 16 bits of address.
    ExtendedLinearAddress(u16),
    /// Start linear address record (type 05): entry point.
    StartLinearAddress(u32),
}

impl Record {
    /// Parse a single Intel HEX record line (including the leading `:`).
    pub fn parse(line: &str) -> Result<Self, MergehexError> {
        let line = line.trim();
        if !line.starts_with(':') {
            return Err(MergehexError::HexParse {
                detail: "record does not start with ':'".into(),
            });
        }
        if line.len() < 11 || line.len() % 2 != 1 {
            return Err(MergehexError::HexParse {
                detail: format!("malformed record length: {}", line),
            });
        }

        let bytes = decode_hex(&line[1..])?;
        let count = bytes[0] as usize;
        if bytes.len() != count + 5 {
            return Err(MergehexError::HexParse {
                detail: format!(
                    "record byte count mismatch: expected {}, got {}",
                    count,
                    bytes.len() - 5
                ),
            });
        }

        let checksum = bytes
            .iter()
            .take(bytes.len() - 1)
            .fold(0u8, |a, &b| a.wrapping_add(b));
        let expected = (!checksum).wrapping_add(1);
        if expected != bytes[bytes.len() - 1] {
            return Err(MergehexError::HexChecksum {
                expected: bytes[bytes.len() - 1],
                computed: expected,
            });
        }

        let offset = u16::from_be_bytes([bytes[1], bytes[2]]);
        let record_type = bytes[3];
        let data = &bytes[4..bytes.len() - 1];

        match record_type {
            0x00 => Ok(Record::Data {
                offset,
                data: data.to_vec(),
            }),
            0x01 => Ok(Record::EndOfFile),
            0x02 => {
                if data.len() != 2 {
                    return Err(MergehexError::HexParse {
                        detail: "extended segment address must contain 2 bytes".into(),
                    });
                }
                Ok(Record::ExtendedSegmentAddress(u16::from_be_bytes([
                    data[0], data[1],
                ])))
            }
            0x03 => {
                if data.len() != 4 {
                    return Err(MergehexError::HexParse {
                        detail: "start segment address must contain 4 bytes".into(),
                    });
                }
                Ok(Record::StartSegmentAddress {
                    cs: u16::from_be_bytes([data[0], data[1]]),
                    ip: u16::from_be_bytes([data[2], data[3]]),
                })
            }
            0x04 => {
                if data.len() != 2 {
                    return Err(MergehexError::HexParse {
                        detail: "extended linear address must contain 2 bytes".into(),
                    });
                }
                Ok(Record::ExtendedLinearAddress(u16::from_be_bytes([
                    data[0], data[1],
                ])))
            }
            0x05 => {
                if data.len() != 4 {
                    return Err(MergehexError::HexParse {
                        detail: "start linear address must contain 4 bytes".into(),
                    });
                }
                Ok(Record::StartLinearAddress(u32::from_be_bytes([
                    data[0], data[1], data[2], data[3],
                ])))
            }
            t => Err(MergehexError::HexParse {
                detail: format!("unsupported record type: 0x{:02X}", t),
            }),
        }
    }
}

/// Decode a string of hex digits into a byte vector.
fn decode_hex(s: &str) -> Result<Vec<u8>, MergehexError> {
    if s.len() % 2 != 0 {
        return Err(MergehexError::HexParse {
            detail: "odd number of hex digits".into(),
        });
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| MergehexError::HexParse {
            detail: format!("invalid hex digit: {}", e),
        })
}

/// Encode a byte slice as a string of uppercase hex digits.
fn encode_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{:02X}", b);
            s
        })
}

/// Parse an Intel HEX file into a flat list of `(address, byte)` pairs.
pub fn parse_hex_file(path: &Path) -> Result<Vec<(u64, u8)>, MergehexError> {
    let file = std::fs::File::open(path).map_err(|e| MergehexError::Io {
        path: path.into(),
        source: e,
    })?;
    let reader = io::BufReader::new(file);

    let mut base_address: u64 = 0;
    let mut segment_address: u64 = 0;
    let mut pairs = Vec::new();
    let mut eof_seen = false;

    for (line_no, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| MergehexError::Io {
            path: path.into(),
            source: e,
        })?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let record = Record::parse(line).map_err(|e| MergehexError::HexLine {
            path: path.into(),
            line: line_no + 1,
            source: Box::new(e),
        })?;

        match record {
            Record::Data { offset, data } => {
                let addr = base_address + segment_address + u64::from(offset);
                for (i, byte) in data.into_iter().enumerate() {
                    pairs.push((addr + i as u64, byte));
                }
            }
            Record::EndOfFile => {
                eof_seen = true;
                break;
            }
            Record::ExtendedSegmentAddress(seg) => {
                segment_address = u64::from(seg) << 4;
                base_address = 0;
            }
            Record::ExtendedLinearAddress(ula) => {
                base_address = u64::from(ula) << 16;
                segment_address = 0;
            }
            // Start addresses do not contribute to the memory image.
            Record::StartSegmentAddress { .. } | Record::StartLinearAddress(_) => {}
        }
    }

    if !eof_seen {
        return Err(MergehexError::HexParse {
            detail: "missing end-of-file record".into(),
        });
    }

    Ok(pairs)
}

/// Write a `MemoryMap` as Intel HEX records to `writer`.
pub fn write_hex<W: Write>(writer: &mut W, memory: &MemoryMap) -> io::Result<()> {
    // Convert the address map into sorted contiguous chunks.
    let data: Vec<(u64, u8)> = memory.iter().collect();
    if data.is_empty() {
        writeln!(writer, ":00000001FF")?;
        return Ok(());
    }

    let mut i = 0;
    while i < data.len() {
        // Start a new contiguous chunk. Emit extended linear address when crossing 64 KiB.
        let mut chunk_start = i;
        let mut current_ula = data[i].0 >> 16;
        write_extended_linear_address_record(writer, current_ula as u16)?;

        while i < data.len() {
            let (addr, _) = data[i];
            let ula = addr >> 16;
            if ula != current_ula {
                current_ula = ula;
                write_extended_linear_address_record(writer, current_ula as u16)?;
            }

            // End the current record when the next byte is discontinuous or
            // when the record length limit is reached.
            let record_start = data[chunk_start].0;
            let mut j = i;
            while j < data.len()
                && data[j].0 == record_start + (j - chunk_start) as u64
                && j - chunk_start < MAX_RECORD_LEN
            {
                j += 1;
            }

            let record_bytes: Vec<u8> = data[chunk_start..j].iter().map(|(_, b)| *b).collect();
            let offset = (data[chunk_start].0 & 0xFFFF) as u16;
            write_data_record(writer, offset, &record_bytes)?;

            if j < data.len() && data[j].0 == data[j - 1].0 + 1 {
                // Continues within same 64 KiB; start next record immediately.
                chunk_start = j;
                i = j;
            } else {
                // Discontinuity or end of chunk; the outer loop will start a new one.
                i = j;
                break;
            }
        }
    }

    writeln!(writer, ":00000001FF")?;
    Ok(())
}

/// Write a single Intel HEX data record.
fn write_data_record<W: Write>(writer: &mut W, offset: u16, data: &[u8]) -> io::Result<()> {
    let mut record = Vec::with_capacity(data.len() + 5);
    record.push(data.len() as u8);
    record.extend_from_slice(&offset.to_be_bytes());
    record.push(0x00); // data record type
    record.extend_from_slice(data);

    let checksum = record.iter().fold(0u8, |a, &b| a.wrapping_add(b));
    record.push((!checksum).wrapping_add(1));

    write!(writer, ":{}", encode_hex(&record))?;
    writeln!(writer)?;
    Ok(())
}

/// Write an extended linear address record (type 04) with a valid checksum.
fn write_extended_linear_address_record<W: Write>(writer: &mut W, ula: u16) -> io::Result<()> {
    let mut record = vec![0x02, 0x00, 0x00, 0x04];
    record.extend_from_slice(&ula.to_be_bytes());
    let checksum = record.iter().fold(0u8, |a, &b| a.wrapping_add(b));
    record.push((!checksum).wrapping_add(1));
    write!(writer, ":{}", encode_hex(&record))?;
    writeln!(writer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_data_record() {
        let rec = Record::parse(":0400000001020304F2").unwrap();
        assert_eq!(
            rec,
            Record::Data {
                offset: 0x0000,
                data: vec![0x01, 0x02, 0x03, 0x04],
            }
        );
    }

    #[test]
    fn parse_eof_record() {
        assert_eq!(Record::parse(":00000001FF").unwrap(), Record::EndOfFile);
    }

    #[test]
    fn parse_extended_linear_address() {
        assert_eq!(
            Record::parse(":020000040800F2").unwrap(),
            Record::ExtendedLinearAddress(0x0800)
        );
    }

    #[test]
    fn checksum_failure_detected() {
        assert!(matches!(
            Record::parse(":100000000C9485000C94A3000C94A3000C94A30000"),
            Err(MergehexError::HexChecksum { .. })
        ));
    }

    #[test]
    fn roundtrip_small_memory_map() {
        let mut memory = MemoryMap::new();
        memory.set(0x0000, 0x01).unwrap();
        memory.set(0x0001, 0x02).unwrap();
        memory.set(0x0002, 0x03).unwrap();
        memory.set(0x0003, 0x04).unwrap();

        let mut output = Vec::new();
        write_hex(&mut output, &memory).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains(":020000040000FA"));
        assert!(text.contains(":0400000001020304F2"));
        assert!(text.contains(":00000001FF"));
    }
}

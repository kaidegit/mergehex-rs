//! Command-line entry point for mergehex-rs.

use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use clap::Parser;

use mergehex_rs::MergehexError;
use mergehex_rs::hex::write_hex;
use mergehex_rs::input::{InputSpec, parse_format};
use mergehex_rs::memory::{MemoryMap, OverlapPolicy};

/// Merge Intel HEX, binary, and ELF files into a single Intel HEX file.
#[derive(Parser, Debug)]
#[command(name = "mergehex-rs")]
#[command(about = "Merge Intel HEX, binary, and ELF files into a single Intel HEX file")]
#[command(version)]
struct Cli {
    /// Input file. Use `file.bin@0xADDR` to specify a binary offset.
    /// Multiple inputs are merged in the given order.
    #[arg(
        short = 'i',
        long = "input",
        required = true,
        value_name = "PATH[@OFFSET]"
    )]
    inputs: Vec<String>,

    /// Output Intel HEX file.
    #[arg(short = 'o', long = "output", required = true, value_name = "PATH")]
    output: PathBuf,

    /// Overlap handling policy.
    #[arg(
        long = "overlap",
        value_name = "POLICY",
        default_value = "error",
        help = "Overlap policy: error, replace, or ignore"
    )]
    overlap: String,

    /// Force a specific input format for all inputs.
    /// When omitted, the format is inferred from each file extension.
    #[arg(long = "format", value_name = "FORMAT")]
    format: Option<String>,
}

fn run() -> Result<(), MergehexError> {
    let cli = Cli::parse();

    let overlap: OverlapPolicy = cli.overlap.parse()?;
    let explicit_format = cli.format.as_deref().map(parse_format).transpose()?;

    let specs: Vec<InputSpec> = cli
        .inputs
        .iter()
        .map(|s| InputSpec::parse(s, explicit_format))
        .collect::<Result<Vec<_>, _>>()?;

    let mut memory = MemoryMap::new();
    for spec in &specs {
        spec.merge_into(&mut memory, overlap)?;
    }

    let output_file = File::create(&cli.output).map_err(|e| MergehexError::Io {
        path: cli.output.clone(),
        source: e,
    })?;
    let mut writer = BufWriter::new(output_file);
    write_hex(&mut writer, &memory).map_err(|e| MergehexError::Io {
        path: cli.output.clone(),
        source: e,
    })?;
    writer.flush().map_err(|e| MergehexError::Io {
        path: cli.output.clone(),
        source: e,
    })?;

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        if let Some(source) = e.source() {
            eprintln!("caused by: {}", source);
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_with_defaults() {
        let cli = Cli::parse_from([
            "mergehex-rs",
            "-i",
            "a.hex",
            "-i",
            "b.bin@0x1000",
            "-o",
            "out.hex",
        ]);
        assert_eq!(cli.inputs, vec!["a.hex", "b.bin@0x1000"]);
        assert_eq!(cli.output, PathBuf::from("out.hex"));
        assert_eq!(cli.overlap, "error");
    }
}

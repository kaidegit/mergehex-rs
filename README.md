# mergehex-rs

[中文说明](README.zh.md)

A cross-platform command-line tool for merging Intel HEX, raw binary, and ELF
files into a single Intel HEX file. Inspired by Nordic Semiconductor's
`mergehex`, but written in Rust and runnable on Linux, Windows, and macOS
(including Apple Silicon).

## Features

- **Input formats**
  - Intel HEX (`.hex`, `.ihex`)
  - Raw binary (`.bin`) with optional load offset (`file.bin@0xADDR`)
  - ELF (`.elf`, `.axf`, `.o`, `.out`) — loadable `PT_LOAD` segments are extracted
- **Output format**: Intel HEX
- **Overlap handling**: `error` (default), `replace`, or `ignore`
- **No external runtime dependencies**: single statically-linkable binary

## Installation

Pre-built binaries for common platforms are available on the
[Releases](https://github.com/mergehex-rs/mergehex-rs/releases) page.

### From source

```bash
cargo install --path .
```

## Usage

```bash
mergehex-rs \
  -i softdevice.hex \
  -i application.hex \
  -i settings.bin@0xFF000 \
  -o merged.hex
```

### Options

| Option | Description |
|--------|-------------|
| `-i, --input <PATH[@OFFSET]>` | Input file. Multiple inputs are merged in the given order. Append `@0xADDR` to binary inputs to set the load address. |
| `-o, --output <PATH>` | Output Intel HEX file. |
| `--overlap <error|replace|ignore>` | How to handle overlapping bytes. Default: `error`. |
| `--format <auto|hex|bin|elf>` | Force the input format for all inputs. By default the format is inferred from the file extension. |
| `-h, --help` | Show help. |
| `-V, --version` | Show version. |

### Examples

Merge a SoftDevice and application:

```bash
mergehex-rs -i s140_nrf52_7.3.0_softdevice.hex -i app.hex -o full.hex
```

Merge with a binary file placed at a specific offset:

```bash
mergehex-rs -i bootloader.hex -i data.bin@0x1000 -o combined.hex
```

Allow later inputs to overwrite earlier ones on overlap:

```bash
mergehex-rs -i a.hex -i b.hex -o out.hex --overlap replace
```

## Development

```bash
# Run tests
cargo test

# Run linter
cargo clippy --all-targets --all-features -- -D warnings

# Build release binary
cargo build --release
```

## Supported CI/CD targets

The GitHub Actions release workflow builds for:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-musl`
- `x86_64-pc-windows-msvc`
- `aarch64-pc-windows-msvc`
- `aarch64-apple-darwin`

## License

Licensed under either of Apache-2.0 or MIT, at your option.

# Agent Notes for mergehex-rs

## Project overview

`mergehex-rs` is a Rust CLI tool that merges firmware images from Intel HEX,
binary, and ELF inputs into a single Intel HEX output. It is designed to be a
cross-platform, dependency-light replacement for Nordic's `mergehex`.

## Build and test

- Toolchain: stable Rust (MSRV 1.85)
- Build: `cargo build --release`
- Test: `cargo test`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Format: `cargo fmt`

## Architecture

- `src/hex.rs` — Intel HEX parser and writer.
- `src/memory.rs` — Sparse `MemoryMap` keyed by absolute address; overlap
  detection and merging.
- `src/input.rs` — Input file format detection and dispatch.
- `src/elf.rs` — ELF loadable segment extraction via the `object` crate.
- `src/main.rs` — `clap` CLI and file I/O.

## Key design decisions

- Overlaps default to `error` to match Nordic `mergehex` behavior. Other
  policies (`replace`, `ignore`) are opt-in.
- Binary offsets use the syntax `file.bin@0xADDR`.
- ELF segments are loaded at `p_paddr` when present, falling back to `p_vaddr`
  (handled by the `object` crate's `Segment::address`).
- Intel HEX output always includes valid checksums and uses extended linear
  address records to support 32-bit address spaces.

## CI/CD

Workflows live in `.github/workflows/`:

- `ci.yml` — runs `cargo fmt`, `cargo clippy`, and `cargo test` on PRs and
  pushes to `main`/`master`.
- `release.yml` — triggered on `v*` tags; builds for 8 targets (Linux GNU/musl
  x86_64/aarch64, Windows x86_64/aarch64, macOS x86_64/aarch64) and creates a
  GitHub Release with artifacts.

Cross-compilation for Linux GNU/musl targets uses `cross`.

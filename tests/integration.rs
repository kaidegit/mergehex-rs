//! Integration tests for the mergehex-rs CLI.

use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mergehex-rs"))
}

#[test]
fn merge_two_hex_files() {
    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.hex");
    let b = dir.path().join("b.hex");
    let out = dir.path().join("out.hex");

    fs::write(&a, ":020000020000FC\n:0400000001020304F2\n:00000001FF\n").unwrap();
    // Place b at 0x10 using extended linear address 0x0000 and offset 0x10.
    fs::write(&b, ":021000000506E3\n:00000001FF\n").unwrap();

    let status = bin()
        .args([
            "-i",
            a.to_str().unwrap(),
            "-i",
            b.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let merged = fs::read_to_string(&out).unwrap();
    assert!(merged.contains(":0400000001020304F2"));
    assert!(merged.contains(":021000000506E3"));
    assert!(merged.contains(":00000001FF"));
}

#[test]
fn merge_hex_and_binary_with_offset() {
    let dir = TempDir::new().unwrap();
    let hex = dir.path().join("fw.hex");
    let bin_file = dir.path().join("data.bin");
    let out = dir.path().join("out.hex");

    fs::write(&hex, ":040000000A0B0C0DCE\n:00000001FF\n").unwrap();
    fs::write(&bin_file, [0x10, 0x11, 0x12, 0x13]).unwrap();

    let status = bin()
        .args([
            "-i",
            hex.to_str().unwrap(),
            "-i",
            &format!("{}@0x1000", bin_file.display()),
            "-o",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let merged = fs::read_to_string(&out).unwrap();
    assert!(merged.contains(":040000000A0B0C0DCE"));
    assert!(merged.contains(":0410000010111213A6"));
}

#[test]
fn overlap_error_by_default() {
    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.hex");
    let b = dir.path().join("b.hex");
    let out = dir.path().join("out.hex");

    fs::write(&a, ":040000000A0B0C0DCE\n:00000001FF\n").unwrap();
    fs::write(&b, ":0400000001020304F2\n:00000001FF\n").unwrap();

    let output = bin()
        .args([
            "-i",
            a.to_str().unwrap(),
            "-i",
            b.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("overlapping data detected"));
}

#[test]
fn overlap_replace_policy() {
    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a.hex");
    let b = dir.path().join("b.hex");
    let out = dir.path().join("out.hex");

    fs::write(&a, ":040000000A0B0C0DCE\n:00000001FF\n").unwrap();
    fs::write(&b, ":0400000001020304F2\n:00000001FF\n").unwrap();

    let status = bin()
        .args([
            "-i",
            a.to_str().unwrap(),
            "-i",
            b.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--overlap",
            "replace",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let merged = fs::read_to_string(&out).unwrap();
    assert!(merged.contains(":0400000001020304F2"));
}

#[test]
fn cli_help_exits_cleanly() {
    let output = bin().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--input"));
    assert!(stdout.contains("--output"));
}

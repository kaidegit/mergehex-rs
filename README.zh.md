# mergehex-rs

[English README](README.md)

一款跨平台命令行工具，用于将 Intel HEX、原始二进制和 ELF 文件合并为单个
Intel HEX 文件。灵感来自 Nordic Semiconductor 的 `mergehex`，但使用 Rust
编写，可在 Linux、Windows 和 macOS（包括 Apple Silicon）上运行。

## 功能特性

- **输入格式**
  - Intel HEX（`.hex`、`.ihex`）
  - 原始二进制（`.bin`），支持通过 `file.bin@0xADDR` 指定加载偏移
  - ELF（`.elf`、`.axf`、`.o`、`.out`）— 自动提取可加载段
- **输出格式**：Intel HEX
- **重叠处理**：`error`（默认）、`replace` 或 `ignore`
- **无外部运行时依赖**：可静态链接为单一二进制文件

## 安装

常用平台的预编译二进制文件可在
[Releases](https://github.com/mergehex-rs/mergehex-rs/releases) 页面获取。

### 从源码安装

```bash
cargo install --path .
```

## 用法

```bash
mergehex-rs \
  -i softdevice.hex \
  -i application.hex \
  -i settings.bin@0xFF000 \
  -o merged.hex
```

### 选项

| 选项 | 说明 |
|------|------|
| `-i, --input <PATH[@OFFSET]>` | 输入文件。按给定顺序合并多个输入。二进制文件可追加 `@0xADDR` 设置加载地址。 |
| `-o, --output <PATH>` | 输出 Intel HEX 文件。 |
| `--overlap <error|replace|ignore>` | 地址重叠时的处理方式。默认：`error`。 |
| `--format <auto|hex|bin|elf>` | 强制指定所有输入的格式。默认按文件扩展名自动推断。 |
| `-h, --help` | 显示帮助信息。 |
| `-V, --version` | 显示版本信息。 |

### 示例

合并 SoftDevice 与应用程序：

```bash
mergehex-rs -i s140_nrf52_7.3.0_softdevice.hex -i app.hex -o full.hex
```

在指定偏移处合并二进制文件：

```bash
mergehex-rs -i bootloader.hex -i data.bin@0x1000 -o combined.hex
```

允许后输入的文件覆盖重叠地址：

```bash
mergehex-rs -i a.hex -i b.hex -o out.hex --overlap replace
```

## 开发

```bash
# 运行测试
cargo test

# 运行代码检查
cargo clippy --all-targets --all-features -- -D warnings

# 构建发布版本
cargo build --release
```

## 支持的 CI/CD 构建目标

GitHub Actions 发布工作流会构建以下目标：

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-musl`
- `x86_64-pc-windows-msvc`
- `aarch64-pc-windows-msvc`
- `aarch64-apple-darwin`

## 许可证

Apache-2.0 或 MIT，可任选其一。

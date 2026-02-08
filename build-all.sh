#!/bin/bash
set -e

# Ensure rustup toolchain is used (not Homebrew)
export PATH="$HOME/.cargo/bin:$PATH"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
STUB_DIR="$SCRIPT_DIR/linux-pcsc-stub"
OUTPUT_DIR="$SCRIPT_DIR/dist"

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

echo "=== Building macOS ARM64 (aarch64-apple-darwin) ==="
cargo build --release --target aarch64-apple-darwin
cp target/aarch64-apple-darwin/release/smart-card-reader "$OUTPUT_DIR/smart-card-reader-macos-arm64"
echo "Done."

echo ""
echo "=== Building macOS x86_64 (x86_64-apple-darwin) ==="
cargo build --release --target x86_64-apple-darwin
cp target/x86_64-apple-darwin/release/smart-card-reader "$OUTPUT_DIR/smart-card-reader-macos-x64"
echo "Done."

echo ""
echo "=== Building Windows x86_64 (x86_64-pc-windows-gnu) ==="
cargo build --release --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/smart-card-reader.exe "$OUTPUT_DIR/smart-card-reader-windows-x64.exe"
echo "Done."

echo ""
echo "=== Building Linux x86_64 (x86_64-unknown-linux-gnu) ==="
PCSC_LIB_DIR="$STUB_DIR" PCSC_LIB_NAME=pcsclite RUSTFLAGS="-L $STUB_DIR" \
  cargo build --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/smart-card-reader "$OUTPUT_DIR/smart-card-reader-linux-x64"
echo "Done."

echo ""
echo "=== All builds complete ==="
echo "Output files:"
ls -lh "$OUTPUT_DIR/"
echo ""
echo "NOTE: Linux binary requires libpcsclite at runtime."
echo "      Install with: sudo apt install pcscd libpcsclite1"

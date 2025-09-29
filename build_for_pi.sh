#!/bin/bash

# Cross-compilation script for Raspberry Pi 5 (ARM64)
# Run this on your development machine to build for Raspberry Pi

set -e

echo "🦀 Cross-compiling Pixel Sorter for Raspberry Pi 5 (ARM64)"
echo "=========================================================="

# Check if cross-compilation target is installed
if ! rustup target list --installed | grep -q "aarch64-unknown-linux-gnu"; then
    echo "📦 Installing ARM64 target for Rust..."
    rustup target add aarch64-unknown-linux-gnu
fi

# Check for cross-compilation dependencies
if ! command -v aarch64-linux-gnu-gcc &> /dev/null; then
    echo "⚠️  Warning: aarch64-linux-gnu-gcc not found"
    echo "   On Ubuntu/Debian: sudo apt install gcc-aarch64-linux-gnu"
    echo "   On macOS: brew install aarch64-elf-gcc"
    echo "   Continuing anyway - may work with system linker..."
fi

# Set environment variables for cross-compilation
export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
export CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
export AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc

# Build for ARM64 (Raspberry Pi 5)
echo "🔨 Building for ARM64 target..."
cargo build --release --target aarch64-unknown-linux-gnu --features gpio

# Check if build was successful
if [ -f "target/aarch64-unknown-linux-gnu/release/pixelsort-pi" ]; then
    echo "✅ Build successful!"
    echo "📁 Binary location: target/aarch64-unknown-linux-gnu/release/pixelsort-pi"
    
    # Get binary size
    SIZE=$(du -h target/aarch64-unknown-linux-gnu/release/pixelsort-pi | cut -f1)
    echo "📊 Binary size: $SIZE"
    
    echo ""
    echo "📋 Next steps:"
    echo "   1. Copy binary to Raspberry Pi:"
    echo "      scp target/aarch64-unknown-linux-gnu/release/pixelsort-pi pi@your-pi-ip:~/pixelsort-pi"
    echo "   2. Copy sample config (optional):"
    echo "      scp pixelsort_config.toml pi@your-pi-ip:~/"
    echo "   3. SSH to Pi and run: ./pixelsort-pi"
    
else
    echo "❌ Build failed!"
    exit 1
fi
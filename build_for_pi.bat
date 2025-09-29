@echo off
REM Build script for Windows - Cross-compile to Raspberry Pi ARM64

echo 🦀 Cross-compiling Pixel Sorter for Raspberry Pi 5 (ARM64)
echo ==========================================================

REM Check if cross-compilation target is installed
rustup target list --installed | findstr "aarch64-unknown-linux-gnu" >nul
if errorlevel 1 (
    echo 📦 Installing ARM64 target for Rust...
    rustup target add aarch64-unknown-linux-gnu
)

REM Set environment variables for cross-compilation
set CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
set CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
set AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar
set CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc

echo 🔨 Building for ARM64 target...
cargo build --release --target aarch64-unknown-linux-gnu --features gpio

REM Check if build was successful
if exist "target\aarch64-unknown-linux-gnu\release\pixelsort-pi.exe" (
    echo ❌ Found .exe extension - this shouldn't happen for Linux target
    echo Check your Rust installation and target configuration
    exit /b 1
)

if exist "target\aarch64-unknown-linux-gnu\release\pixelsort-pi" (
    echo ✅ Build successful!
    echo 📁 Binary location: target\aarch64-unknown-linux-gnu\release\pixelsort-pi
    
    echo.
    echo 📋 Next steps:
    echo    1. Copy binary to Raspberry Pi:
    echo       scp target/aarch64-unknown-linux-gnu/release/pixelsort-pi pi@your-pi-ip:~/pixelsort-pi
    echo    2. Copy sample config (optional^):
    echo       scp pixelsort_config.toml pi@your-pi-ip:~/
    echo    3. SSH to Pi and run: ./pixelsort-pi
    
) else (
    echo ❌ Build failed!
    echo Make sure you have the cross-compilation toolchain installed
    echo On Windows, you might need WSL or cross-compilation tools from:
    echo https://github.com/cross-rs/cross
    exit /b 1
)

echo.
echo 💡 Tip: If cross-compilation fails on Windows, consider using:
echo    1. WSL (Windows Subsystem for Linux^)
echo    2. Docker with cross-compilation image
echo    3. Build directly on the Raspberry Pi
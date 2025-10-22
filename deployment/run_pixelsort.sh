#!/bin/bash
# Raspberry Pi Pixel Sorter - Auto-update launcher script
# This script updates the app from git and runs it

APP_DIR="/home/pixelsort/Pixelsort"
REPO_URL="https://github.com/Vaghabund/Pixelsort.git"

cd "$APP_DIR" || exit 1

echo "=========================================="
echo "Raspberry Pi Pixel Sorter - Starting..."
echo "=========================================="

# Check for internet connectivity
if ping -c 1 github.com &> /dev/null; then
    echo "Internet connected. Checking for updates..."
    
    # Fetch latest changes
    git fetch origin main
    
    # Check if we're behind
    LOCAL=$(git rev-parse HEAD)
    REMOTE=$(git rev-parse origin/main)
    
    if [ "$LOCAL" != "$REMOTE" ]; then
        echo "Updates found! Pulling changes..."
        git pull origin main
    else
        echo "Already up to date."
    fi
else
    echo "No internet connection. Skipping update check."
fi

echo "Starting Pixel Sorter..."
echo "=========================================="

# Check if cargo is available for rebuilding if needed
if command -v cargo &> /dev/null; then
    # Rebuild only if source files changed
    cargo build --release
    BINARY="$APP_DIR/target/release/pixelsort-pi"
else
    # No cargo available, use existing binary
    BINARY="$APP_DIR/target/release/pixelsort-pi"
    if [ ! -f "$BINARY" ]; then
        echo "ERROR: Binary not found and cargo not available!"
        exit 1
    fi
fi

# Run the application
"$BINARY"

# If app exits, wait a moment before this script ends
echo ""
echo "Application closed."
sleep 2

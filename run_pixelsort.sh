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
        
        echo "Rebuilding application (this may take a few minutes)..."
        cargo build --release
        
        if [ $? -eq 0 ]; then
            echo "Build successful!"
        else
            echo "Build failed! Using existing version."
        fi
    else
        echo "Already up to date."
    fi
else
    echo "No internet connection. Skipping update check."
fi

echo "Starting Pixel Sorter..."
echo "=========================================="

# Run the application (release mode for better performance)
cargo run --release

# If app exits, wait a moment before this script ends
echo ""
echo "Application closed."
sleep 2

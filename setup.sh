#!/bin/bash

# Raspberry Pi Pixel Sorter Setup Script
# Run this script to set up the pixel sorter on your Raspberry Pi

echo "🎨 Raspberry Pi Pixel Sorter Setup"
echo "=================================="

# Check if running on Raspberry Pi
if ! grep -q "Raspberry Pi" /proc/cpuinfo 2>/dev/null; then
    echo "⚠️  Warning: This doesn't appear to be a Raspberry Pi"
    echo "   The GPIO functionality will not work on this system"
    read -p "   Continue anyway? (y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Update system
echo "📦 Updating system packages..."
sudo apt update

# Install system dependencies
echo "📦 Installing system dependencies..."
sudo apt install -y python3-pip python3-tk python3-pil python3-numpy python3-rpi.gpio

# Install Python packages
echo "🐍 Installing Python packages..."
pip3 install -r requirements.txt

# Create sample images directory
echo "🖼️  Setting up sample images..."
mkdir -p sample_images
python3 -c "from image_processor import ImageProcessor; ImageProcessor().create_sample_images()"

# Make main script executable
chmod +x main.py

# Test GPIO (if available)
if command -v gpio &> /dev/null; then
    echo "🔌 Testing GPIO access..."
    python3 -c "
try:
    import RPi.GPIO as GPIO
    GPIO.setmode(GPIO.BCM)
    GPIO.cleanup()
    print('✅ GPIO access successful')
except Exception as e:
    print(f'❌ GPIO test failed: {e}')
"
fi

echo ""
echo "✅ Setup complete!"
echo ""
echo "📋 Next steps:"
echo "   1. Connect your 7-inch display via HDMI"
echo "   2. Wire GPIO buttons according to README.md"
echo "   3. Run: python3 main.py"
echo ""
echo "🔧 Optional configuration:"
echo "   - Edit config.py for display/GPIO settings"
echo "   - Add to autostart (see README.md)"
echo ""
echo "🚀 Ready to sort some pixels!"
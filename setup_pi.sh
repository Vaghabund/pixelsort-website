#!/bin/bash

# Raspberry Pi setup and installation script
# Run this on your Raspberry Pi to set up the pixel sorter

set -e

echo "🎨 Raspberry Pi Pixel Sorter Setup"
echo "=================================="

# Check if running on Raspberry Pi
if grep -q "Raspberry Pi" /proc/cpuinfo 2>/dev/null; then
    PI_MODEL=$(grep "Model" /proc/cpuinfo | cut -d ':' -f 2 | xargs)
    echo "✅ Running on: $PI_MODEL"
else
    echo "⚠️  Warning: This doesn't appear to be a Raspberry Pi"
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
sudo apt install -y \
    build-essential \
    pkg-config \
    libfontconfig1-dev \
    libfreetype6-dev \
    libx11-dev \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libssl-dev

# Install Rust if not already installed
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
else
    echo "✅ Rust is already installed"
fi

# Build the application (if source code is present)
if [ -f "Cargo.toml" ]; then
    echo "🔨 Building pixel sorter from source..."
    cargo build --release --features gpio
    
    # Create binary symlink for easy access
    sudo ln -sf $(pwd)/target/release/pixelsort-pi /usr/local/bin/pixelsort-pi
    
    echo "✅ Built from source successfully"
else
    echo "ℹ️  No source code found - assuming binary will be provided separately"
fi

# Create directories
echo "📁 Creating directories..."
mkdir -p ~/pixelsort
mkdir -p ~/pixelsort/sample_images
mkdir -p ~/pixelsort/output

# Create desktop entry for GUI launch
echo "🖥️  Creating desktop entry..."
cat > ~/.local/share/applications/pixelsort-pi.desktop << EOF
[Desktop Entry]
Name=Pixel Sorter
Comment=Manipulatable pixel sorting for Raspberry Pi
Exec=/usr/local/bin/pixelsort-pi
Icon=applications-graphics
Terminal=false
Type=Application
Categories=Graphics;Photography;
EOF

# Create default configuration
echo "⚙️  Creating default configuration..."
cat > ~/pixelsort/pixelsort_config.toml << EOF
[display]
width = 800
height = 480
fullscreen = true
image_display_width = 480
image_display_height = 360

[gpio]
enabled = true
debounce_ms = 200

[gpio.pins]
load_image = 18
next_algorithm = 19
threshold_up = 20
threshold_down = 21
save_image = 26

[processing]
default_threshold = 50.0
default_interval = 10
max_image_width = 1920
max_image_height = 1080
preview_scale_factor = 4

[paths]
sample_images_dir = "sample_images"
default_save_dir = "output"
config_file = "pixelsort_config.toml"
EOF

# Set up permissions for GPIO access
echo "🔌 Setting up GPIO permissions..."
sudo usermod -a -G gpio $USER

# Create systemd service for auto-start (optional)
read -p "🚀 Create auto-start service? (y/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "📝 Creating systemd service..."
    sudo tee /etc/systemd/system/pixelsort-pi.service > /dev/null << EOF
[Unit]
Description=Pixel Sorter Application
After=graphical-session.target
Wants=graphical-session.target

[Service]
Type=simple
User=$USER
Group=$USER
Environment=DISPLAY=:0
Environment=XDG_RUNTIME_DIR=/run/user/$(id -u $USER)
WorkingDirectory=/home/$USER/pixelsort
ExecStart=/usr/local/bin/pixelsort-pi
Restart=always
RestartSec=5

[Install]
WantedBy=graphical-session.target
EOF

    sudo systemctl daemon-reload
    sudo systemctl enable pixelsort-pi.service
    
    echo "✅ Auto-start service created and enabled"
fi

# Create test script
echo "🧪 Creating test script..."
cat > ~/pixelsort/test_gpio.sh << 'EOF'
#!/bin/bash
echo "Testing GPIO button connections..."
echo "Press Ctrl+C to exit"

# Simple GPIO test using rppal (if available)
python3 << 'PYTHON'
import RPi.GPIO as GPIO
import time
import signal
import sys

# GPIO pins for buttons
PINS = {
    18: "Load Image",
    19: "Next Algorithm", 
    20: "Threshold Up",
    21: "Threshold Down",
    26: "Save Image"
}

def signal_handler(sig, frame):
    GPIO.cleanup()
    print("\nGPIO cleanup complete")
    sys.exit(0)

# Setup
GPIO.setmode(GPIO.BCM)
GPIO.setwarnings(False)

for pin in PINS.keys():
    GPIO.setup(pin, GPIO.IN, pull_up_down=GPIO.PUD_UP)

signal.signal(signal.SIGINT, signal_handler)

print("GPIO test ready. Press buttons to test...")

try:
    while True:
        for pin, name in PINS.items():
            if not GPIO.input(pin):  # Button pressed (active low)
                print(f"Button pressed: {name} (GPIO {pin})")
                time.sleep(0.3)  # Debounce
        time.sleep(0.1)
        
except KeyboardInterrupt:
    pass
finally:
    GPIO.cleanup()
PYTHON
EOF

chmod +x ~/pixelsort/test_gpio.sh

echo ""
echo "✅ Setup complete!"
echo ""
echo "📋 What's been set up:"
echo "   • System dependencies installed"
echo "   • Rust toolchain (if needed)"
echo "   • Default configuration created"
echo "   • GPIO permissions configured" 
echo "   • Desktop entry created"
echo "   • Test script created"
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "   • Auto-start service enabled"
fi
echo ""
echo "🔧 Next steps:"
echo "   1. Wire your GPIO buttons according to the pin configuration"
echo "   2. Test GPIO: ~/pixelsort/test_gpio.sh"
echo "   3. Copy/build the pixelsort-pi binary"
echo "   4. Run: pixelsort-pi"
echo ""
echo "📍 Important files:"
echo "   • Configuration: ~/pixelsort/pixelsort_config.toml"
echo "   • GPIO test: ~/pixelsort/test_gpio.sh"
echo "   • Sample images: ~/pixelsort/sample_images/"
echo ""
echo "🔄 Reboot required for GPIO permissions to take effect"
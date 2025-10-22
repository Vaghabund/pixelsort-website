#!/bin/bash
# Setup script for Raspberry Pi Pixel Sorter - Auto-start on boot

echo "=========================================="
echo "Raspberry Pi Pixel Sorter - Setup"
echo "=========================================="

APP_DIR="/home/pixelsort/Pixelsort"
SERVICE_FILE="pixelsort-kiosk.service"

# Check if we're in the right directory
if [ ! -f "run_pixelsort.sh" ]; then
    echo "Error: run_pixelsort.sh not found. Please run this script from the Pixelsort directory."
    exit 1
fi

echo "1. Making launcher script executable..."
chmod +x run_pixelsort.sh

echo "2. Testing launcher script permissions..."
if [ -x "run_pixelsort.sh" ]; then
    echo "   ✓ Launcher script is executable"
else
    echo "   ✗ Failed to make launcher executable"
    exit 1
fi

echo ""
echo "3. Installing systemd service for auto-start on boot..."
sudo cp "$SERVICE_FILE" /etc/systemd/system/

echo "4. Reloading systemd daemon..."
sudo systemctl daemon-reload

echo "5. Enabling service to start on boot..."
sudo systemctl enable pixelsort-kiosk.service

echo ""
echo "=========================================="
echo "Setup complete!"
echo "=========================================="
echo ""
echo "The Pixel Sorter will now start automatically on boot."
echo ""
echo "Useful commands:"
echo "  Start now:   sudo systemctl start pixelsort-kiosk.service"
echo "  Stop:        sudo systemctl stop pixelsort-kiosk.service"
echo "  Status:      sudo systemctl status pixelsort-kiosk.service"
echo "  View logs:   journalctl -u pixelsort-kiosk.service -f"
echo "  Disable:     sudo systemctl disable pixelsort-kiosk.service"
echo ""
echo "To start the service now, run:"
echo "  sudo systemctl start pixelsort-kiosk.service"
echo ""

# Raspberry Pi Pixel Sorter - Deployment Guide

Complete guide for deploying the Pixel Sorter app on Raspberry Pi 5.

## Prerequisites

### Hardware:
- Raspberry Pi 5
- 7-inch TFT touchscreen (HDMI connected)
- Raspberry Pi Camera Module (v1.5 or later)
- 16GB+ microSD card
- Optional: USB drive for exporting images

### Software:
- Raspberry Pi OS (64-bit, Bookworm or later)
- Rust toolchain
- rpicam tools (`rpicam-vid`, `rpicam-still`)

---

## Quick Install (On Raspberry Pi)

### 1. Install Dependencies

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install camera tools
sudo apt install -y rpicam-apps

# Install build dependencies for egui/eframe
sudo apt install -y libgtk-3-dev libglib2.0-dev libcairo2-dev \
    libpango1.0-dev libgdk-pixbuf2.0-dev libatk1.0-dev
```

### 2. Clone and Build

```bash
# Clone repository
git clone https://github.com/Vaghabund/Pixelsort.git
cd Pixelsort

# Build release version (takes 5-10 minutes on Pi)
cargo build --release

# Test run
./target/release/pixelsort-pi
```

---

## Auto-Start on Boot (Kiosk Mode)

To make the app start automatically when the Pi boots:

### 1. Run Setup Script

```bash
cd ~/Pixelsort/deployment
chmod +x setup_autostart.sh
./setup_autostart.sh
```

### 2. Enable Auto-Login

```bash
sudo raspi-config
```

Navigate to:
- **System Options** → **Boot / Auto Login**
- Select: **Desktop Autologin**
- Reboot

### 3. Done!

The app will now start automatically on boot in fullscreen kiosk mode.

---

## Auto-Update on Launch

The deployment includes an auto-update launcher that:
1. Checks for git updates from GitHub
2. Pulls latest changes if available
3. Rebuilds only changed files
4. Launches the app

The systemd service uses `run_pixelsort.sh` which handles this automatically.

---

## Exit Methods

### Keyboard:
- Press **ESC**

### Touchscreen:
- Tap top-left corner **5 times** within 3 seconds

---

## Manual Control

```bash
# Start service
sudo systemctl start pixelsort-kiosk.service

# Stop service
sudo systemctl stop pixelsort-kiosk.service

# Check status
sudo systemctl status pixelsort-kiosk.service

# View live logs
journalctl -u pixelsort-kiosk.service -f

# Disable auto-start
sudo systemctl disable pixelsort-kiosk.service

# Re-enable auto-start
sudo systemctl enable pixelsort-kiosk.service
```

---

## File Locations

| File/Folder | Location | Purpose |
|------------|----------|---------|
| Application binary | `/home/pixelsort/Pixelsort/target/release/pixelsort-pi` | Compiled executable |
| Launcher script | `/home/pixelsort/Pixelsort/deployment/run_pixelsort.sh` | Auto-update wrapper |
| Systemd service | `/etc/systemd/system/pixelsort-kiosk.service` | Boot service |
| Sorted images | `/home/pixelsort/Pixelsort/sorted_images/` | Output directory |
| Application icon | `/home/pixelsort/Pixelsort/assets/Harpy_ICON.png` | UI assets |

---

## Troubleshooting

### App won't start on boot:
```bash
# Check service logs
journalctl -u pixelsort-kiosk.service -n 50

# Test launcher manually
cd ~/Pixelsort/deployment
./run_pixelsort.sh
```

### Camera not working:
```bash
# Test camera
rpicam-hello

# Check camera detection
vcgencmd get_camera
```

### Display issues:
```bash
# Force resolution in /boot/config.txt
sudo nano /boot/config.txt

# Add:
hdmi_force_hotplug=1
hdmi_group=2
hdmi_mode=87
hdmi_cvt=1920 1080 60 6 0 0 0
```

### Build fails:
```bash
# Clean and rebuild
cargo clean
cargo build --release
```

### Permission issues:
```bash
# Fix ownership
sudo chown -R pixelsort:pixelsort /home/pixelsort/Pixelsort
```

---

## USB Export

The app automatically detects USB drives mounted under:
- `/media/pi/`
- `/media/usb/`
- `/media/`
- `/mnt/usb/`
- `/mnt/`

To use:
1. Insert USB drive
2. Wait for auto-mount
3. In Edit phase, press **USB** button if visible
4. Images copied to `USB:/pixelsort_export/`

---

## Advanced Configuration

### Change display resolution:

Edit `src/main.rs`:
```rust
.with_inner_size([1920.0, 1080.0])  // Change resolution here
```

Then rebuild:
```bash
cargo build --release
```

### Adjust sleep timeout:

Edit `src/ui.rs`:
```rust
if !self.is_sleeping && idle_duration >= 300 {  // 300 = 5 minutes
```

### Modify button sizes:

Edit `src/ui.rs` constants:
```rust
const BUTTON_RADIUS: f32 = 100.0;  // Adjust size
```

---

## Uninstall

```bash
# Stop and disable service
sudo systemctl stop pixelsort-kiosk.service
sudo systemctl disable pixelsort-kiosk.service
sudo rm /etc/systemd/system/pixelsort-kiosk.service
sudo systemctl daemon-reload

# Remove application
rm -rf ~/Pixelsort

# Disable auto-login (if desired)
sudo raspi-config
# System Options → Boot / Auto Login → Console
```

---

## Support

- **Issues**: https://github.com/Vaghabund/Pixelsort/issues
- **Discussions**: https://github.com/Vaghabund/Pixelsort/discussions

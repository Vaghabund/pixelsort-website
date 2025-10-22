# Auto-Update Launcher Setup

This setup allows the Pixel Sorter app to automatically check for updates on GitHub and rebuild before starting.

## How It Works

1. **Wrapper Script**: `run_pixelsort.sh` checks for git updates before launching the app
2. **Update Check**: Compares local and remote git commits
3. **Auto Pull**: If updates found, pulls changes from GitHub
4. **Smart Build**: `cargo run --release` automatically rebuilds only what changed
5. **Launch**: Starts the app

## Installation on Raspberry Pi

### 1. Make the script executable:
```bash
cd ~/Pixelsort
chmod +x run_pixelsort.sh
```

### 2. Test manual launch:
```bash
./run_pixelsort.sh
```

### 3. (Optional) Set up auto-start on boot:

Copy the systemd service file:
```bash
sudo cp pixelsort-kiosk.service /etc/systemd/system/
```

Enable and start the service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable pixelsort-kiosk.service
sudo systemctl start pixelsort-kiosk.service
```

Check status:
```bash
sudo systemctl status pixelsort-kiosk.service
```

View logs:
```bash
journalctl -u pixelsort-kiosk.service -f
```

Stop the service:
```bash
sudo systemctl stop pixelsort-kiosk.service
```

### 4. Disable auto-start (if needed):
```bash
sudo systemctl disable pixelsort-kiosk.service
```

## Usage on Windows (Development)

Just run the PowerShell script:
```powershell
.\run_pixelsort.ps1
```

## Notes

- **Smart compilation**: `cargo run` only rebuilds changed files, not the entire project
- **First run**: Initial compilation takes 1-2 minutes on Raspberry Pi
- **Updates**: Subsequent builds are much faster (usually seconds, not minutes)
- **No internet**: App starts normally if GitHub is unreachable
- **Release mode**: Uses `--release` flag for optimal performance

## Exiting the App

- **Keyboard**: Press `ESC`
- **Touchscreen**: Tap top-left corner 5 times within 3 seconds

## Manual Git Update

If you need to manually update without the launcher:
```bash
cd ~/Pixelsort
git pull origin main
cargo build --release
cargo run --release
```

## Troubleshooting

### Service won't start:
```bash
# Check logs
journalctl -u pixelsort-kiosk.service -n 50

# Verify script is executable
ls -la /home/pixelsort/Pixelsort/run_pixelsort.sh

# Test script manually
/home/pixelsort/Pixelsort/run_pixelsort.sh
```

### Build fails:
```bash
# Clean and rebuild
cd ~/Pixelsort
cargo clean
cargo build --release
```

### Permission issues:
```bash
# Fix ownership
sudo chown -R pixelsort:pixelsort /home/pixelsort/Pixelsort
```

# Raspberry Pi Auto-Start Setup

Setup the Pixel Sorter to automatically start on boot in kiosk mode.

## Quick Setup

Run the setup script on your Raspberry Pi:

```bash
cd ~/Pixelsort
chmod +x setup_autostart.sh
./setup_autostart.sh
```

That's it! The app will now start automatically when the Pi boots.

## What It Does

1. Makes `run_pixelsort.sh` executable
2. Installs systemd service
3. Enables auto-start on boot
4. The launcher script handles git updates and launches the app

## Boot Directly Into App (Skip Desktop)

To make the Pi boot directly into the app without showing the desktop:

1. Enable auto-login:
   ```bash
   sudo raspi-config
   ```
   - Navigate to: **System Options** → **Boot / Auto Login**
   - Select: **Desktop Autologin**
   - Exit and reboot

2. The systemd service will automatically start the app after desktop loads

## Boot Sequence

When the Pi boots:
1. Desktop environment loads (briefly visible)
2. Service starts → runs `run_pixelsort.sh`
3. Script checks for updates (if internet available)
4. Splash screen appears (2 seconds)
5. App launches in fullscreen kiosk mode covering the desktop

## Manual Control

Start the service manually:
```bash
sudo systemctl start pixelsort-kiosk.service
```

Stop the service:
```bash
sudo systemctl stop pixelsort-kiosk.service
```

Check status:
```bash
sudo systemctl status pixelsort-kiosk.service
```

View live logs:
```bash
journalctl -u pixelsort-kiosk.service -f
```

Disable auto-start:
```bash
sudo systemctl disable pixelsort-kiosk.service
```

## Exit the App

- **Keyboard**: Press `ESC`
- **Touchscreen**: Tap top-left corner 5 times within 3 seconds

## Troubleshooting

### Service won't start

Check logs for errors:
```bash
journalctl -u pixelsort-kiosk.service -n 50
```

### Permission issues

Fix ownership:
```bash
sudo chown -R pixelsort:pixelsort /home/pixelsort/Pixelsort
```

### Test manually

Run the script directly to see any errors:
```bash
cd ~/Pixelsort
./run_pixelsort.sh
```

### Display not working

Make sure the service has access to the display:
```bash
# Add your user to the display group
sudo usermod -a -G video pixelsort
```

Verify DISPLAY environment variable:
```bash
echo $DISPLAY
# Should show ":0" or similar
```

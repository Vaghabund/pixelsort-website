# Harpy Pixel Sorter

Touch-optimized pixel sorting handheld device built on Raspberry Pi 5 (7" TFT via HDMI) with a clean, responsive egui UI. Build and iterate effects fast by sorting pixels horizontally, vertically, or diagonally.

**Harpy** is a portable, kiosk-mode pixel sorting device designed for creative image manipulation through touch interaction.

## Why Rust?

- Fast native performance for real-time interaction
- Low memory footprint and predictable behavior
- Memory safety without garbage collection
- Portable development on desktop; deploy to Pi

## Features

### Core Functionality
- Live camera preview on Pi (rpicam-vid) with one-tap capture
- 3 algorithms: Horizontal, Vertical, Diagonal
- Edit phase with two controls:
  - Threshold slider (sensitivity of segment breaks)
  - Hue slider for optional tint (display-only)
- Crop phase with draggable handles; apply to turn crop into the new image
- Save & Iterate pipeline: auto-saves to `sorted_images/session_YYYYMMDD_HHMMSS/edit_XXX_*.png` and loads the last save as the new source
- USB export: copies entire `sorted_images/` to any mounted USB under `/media/*` or `/mnt/*`

### Touch-Optimized UI
- Large circular buttons (100-120px radius) for easy touch interaction
- Semi-transparent design with alpha blending for modern look
- Wide vertical sliders with oversized handles (0.8x width)
- Triple spacing between controls for fat-finger friendliness
- Hidden cursor in kiosk mode

### Production Features
- **Kiosk Mode**: Fullscreen operation at 1920x1080 with no window decorations
- **Sleep Mode**: Automatically dims to logo screen after 5 minutes of inactivity; touch anywhere to wake
- **Splash Screen**: 2-second fade-in logo on startup
- **Auto-Update**: Launcher script checks for updates from GitHub before starting
- **Auto-Start**: Systemd service for automatic launch on boot
- **Dual Exit Methods**: ESC key or 5 rapid taps in top-left corner (for touchscreen)

### Development
- Works on desktop for development (Windows/macOS/Linux) with animated test pattern when camera is unavailable
- Full UI functionality on any platform
- Cross-compilation support via `cross` tool

## About Harpy

**Harpy** is designed as a self-contained creative tool - a handheld pixel sorting device that artists can use anywhere. The kiosk mode and touch-optimized interface make it feel like a dedicated hardware device rather than a general-purpose computer.

### Development Philosophy

This is a **vibe coding project** - built through natural collaboration between human creativity and AI assistance. GitHub Copilot has been an integral part of the development process, helping to architect the touch-optimized UI, implement the kiosk mode features, and refine the user experience. The result is a tool that combines artistic vision with rapid iterative development, creating something that feels both polished and experimental.

## Quick Start

### Development (Desktop)

Requirements:
- Rust (stable)
- No additional dependencies

```powershell
# Windows/macOS/Linux
cargo run
```

Camera functionality is disabled on desktop; you'll see an animated test pattern instead.

### Deployment (Harpy Device / Raspberry Pi)

See the comprehensive [Deployment Guide](deployment/README.md) for full setup instructions.

**Quick Install:**
```bash
# Clone and build on your Harpy device
git clone https://github.com/Vaghabund/Pixelsort.git
cd Pixelsort
cargo build --release

# Set up auto-start to make it feel like a dedicated device
cd deployment
sudo ./setup_autostart.sh
```

**Manual Run:**
```bash
./deployment/run_pixelsort.sh
```

The launcher script automatically checks for updates from GitHub before starting.

### Cross-Compilation (Recommended)

Build for Raspberry Pi from your development machine:

```powershell
# Install cross once
cargo install cross

# Ensure Docker is running, then build
cross build --release --target aarch64-unknown-linux-gnu
```

Or use the provided scripts:
- `build_for_pi.sh` (Linux/macOS)
- `build_for_pi.bat` (Windows)

The binary will be at: `target/aarch64-unknown-linux-gnu/release/pixelsort-pi`

## UI Flow

- Input: Take Picture, Upload Image
- Edit: threshold + hue sliders; buttons for Algorithm, Sort Mode, Crop, Save & Iterate, New Image; optional Export to USB row when a drive is mounted
- Crop: drag corner handles; Apply Crop or Cancel

Notes
- Tint is applied as a display effect after sorting (doesn't change the source pixels until saved via Save & Iterate)
- Algorithm and Sort Mode cycle through predefined values

## Project Structure

```
src/
  main.rs               # App entry, window config, kiosk mode setup
  ui.rs                 # Three-phase UI (Input/Edit/Crop), touch controls
  pixel_sorter.rs       # Sorting algorithms + threshold/hue processing
  camera_controller.rs  # rpicam streaming (30 FPS) and snapshot capture
  session.rs            # Auto-save workflow, USB export
  crop.rs               # Crop rectangle logic and application
  image_ops.rs          # Image loading, tint blending, sort integration
  camera.rs             # Camera UI integration
  texture.rs            # egui texture helpers for 30 FPS optimization

deployment/
  run_pixelsort.sh      # Auto-update launcher (checks git, rebuilds)
  run_pixelsort.ps1     # Windows version for development
  setup_autostart.sh    # Systemd service installer
  pixelsort-kiosk.service # Systemd unit file
  README.md             # Complete deployment guide

assets/
  Harpy_ICON.png        # Application icon (splash + sleep screens)

sorted_images/          # Output directory (git-ignored)
  session_YYYYMMDD_HHMMSS/
    edit_001_horizontal.png
    edit_002_vertical.png
    ...
```

## Kiosk Mode Details

When running on the Harpy device (Raspberry Pi), the app operates in kiosk mode to feel like a dedicated handheld:
- Fullscreen at 1920x1080 resolution
- No window decorations or title bar
- Cursor hidden for clean touch interaction
- Sleep mode activates after 5 minutes idle (shows dim Harpy logo)
- Touch anywhere to wake from sleep

**Exit Methods:**
- Press ESC key (if keyboard connected)
- Tap top-left corner 5 times rapidly (within 3 seconds)

## Troubleshooting

### Development
- **No camera on desktop**: App shows animated test pattern; capture button has no effect (this is normal)
- **Window size**: Runs at 1920x1080 on Pi; resizable on desktop for testing

### Harpy Device (Raspberry Pi)
- **Camera not working**: Install rpicam tools with `sudo apt install -y rpicam-apps`
- **USB export**: Requires a mounted drive under `/media/*` or `/mnt/*`; copies entire `sorted_images/` directory
- **Resolution issues**: App forces 1920x1080 with zoom factor 1.0 to disable DPI scaling
- **Auto-start not working**: Check systemd service status with `systemctl --user status pixelsort-kiosk`
- **Can't exit**: Use ESC key or 5 rapid taps in top-left corner

For more troubleshooting, see the [Deployment Guide](deployment/README.md).

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Version History

See [CHANGELOG.md](CHANGELOG.md) for release notes and version history.

## Contributing

Issues and PRs are welcome! Please ensure your code:
- Follows Rust idioms and formatting (`cargo fmt`)
- Passes all checks (`cargo clippy`)
- Is tested on desktop before submitting

## Links

- [Deployment Guide](deployment/README.md) - Complete setup instructions for Raspberry Pi
- [Copilot Instructions](.github/copilot-instructions.md) - Project architecture and development guidelines
- [GitHub Repository](https://github.com/Vaghabund/Pixelsort)
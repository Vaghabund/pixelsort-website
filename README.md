# Raspberry Pi Pixel Sorter (Rust)

Touch-optimized pixel sorting app for Raspberry Pi 5 (7" TFT via HDMI) with a clean, responsive egui UI. Build and iterate effects fast by sorting pixels horizontally, vertically, or diagonally.

## ✨ Features

- Live camera preview on Pi (rpicam-vid) with one-tap capture
- 3 algorithms: Horizontal, Vertical, Diagonal
- Edit phase with two controls:
  - Threshold slider (sensitivity of segment breaks)
  - Hue slider for optional tint (display-only)
- Crop phase with draggable handles; apply to turn crop into the new image
- Save & Iterate pipeline: auto-saves to `sorted_images/session_YYYYMMDD_HHMMSS/edit_XXX_*.png` and loads the last save as the new source
- USB export: copies entire `sorted_images/` to any mounted USB under `/media/*` or `/mnt/*`
- Works on desktop for development (Windows/macOS/Linux) with animated test pattern when camera is unavailable

## 📦 Build and Run

Requirements:
- Rust (stable)
- On Windows/macOS/Linux (dev): nothing else required
- On Raspberry Pi: rpicam tools installed (`rpicam-vid`, `rpicam-still`)

Windows/macOS/Linux (development):
```powershell
cargo run
```

Raspberry Pi (on device):
```bash
cargo build --release
./target/release/pixelsort-pi
```

Cross-compile for Pi (from PC):
- Scripts are provided: `build_for_pi.sh` (Linux/macOS) and `build_for_pi.bat` (Windows)
- Or use the included `Cross.toml` if you prefer cross

## 🖱️ UI flow

- Input: Take Picture, Upload Image
- Edit: threshold + hue sliders; buttons for Algorithm, Sort Mode, Crop, Save & Iterate, New Image; optional Export to USB row when a drive is mounted
- Crop: drag corner handles; Apply Crop or Cancel

Notes
- Tint is applied as a display effect after sorting (doesn’t change the source pixels until saved via Save & Iterate)
- Algorithm and Sort Mode cycle through predefined values

## 🗂️ Project structure

```
src/
  main.rs             # App entry + window setup
  ui.rs               # Phases, layout, touch sliders, rendering
  pixel_sorter.rs     # Sorting algorithms + utilities
  image_ops.rs        # Sorting/tint integration and image loading
  crop.rs             # Crop application logic
  session.rs          # Auto-save and session iteration; USB export
  camera_controller.rs# Pi camera streaming and snapshot via rpicam
  camera.rs           # UI glue for capture flow
  texture.rs          # egui texture helpers
sorted_images/        # Generated sessions (git-ignored)
```

Removed legacy modules: config.rs, image_processor.rs

## 🔧 Troubleshooting

- No camera on desktop: App shows animated test pattern; capture button has no effect
- USB export: requires a mounted drive under `/media/*` or `/mnt/*`; copies the entire `sorted_images/` directory
- Window size: starts at 1024x600 with minimum 800x480; resizable on desktop

## 📝 License

MIT

## 🙌 Contributing

Issues and PRs are welcome.
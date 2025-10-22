# Harpy Pixel Sorter Project

**Harpy** is a touch-optimized pixel sorting handheld device built on Raspberry Pi 5 with 7-inch TFT touchscreen. Designed to feel like a dedicated creative tool rather than a general-purpose computer.

## Project Overview
- **Language**: Rust (2021 edition)
- **Target Platform**: Raspberry Pi 5 (64-bit ARM)
- **Display**: 7-inch TFT touchscreen, 1920x1080 via HDMI
- **GUI Framework**: eframe/egui 0.24 (immediate mode GUI)
- **Camera**: Raspberry Pi Camera Module (rpicam integration)
- **Deployment**: Kiosk mode with auto-start on boot

## 🏗️ Architecture

### Core Components:
- **main.rs**: Application entry, window setup, icon loading, kiosk mode config
- **ui.rs**: Three-phase UI (Input/Edit/Crop), circular touch buttons, vertical sliders, phase transitions
- **pixel_sorter.rs**: Sorting algorithms (Horizontal/Vertical/Diagonal), threshold/hue processing
- **camera_controller.rs**: rpicam streaming (30 FPS), snapshot capture, test pattern fallback
- **session.rs**: Auto-save workflow, session management, USB export
- **crop.rs**: Crop rectangle manipulation, apply crop with sorting
- **image_ops.rs**: Image loading, tint application, pixel sort integration
- **texture.rs**: egui texture management, optimization for 30 FPS preview

### UI Phases:
1. **Input Phase**: Camera preview, Take Picture (120px), Upload (60px) buttons
2. **Edit Phase**: Processed image, Algorithm/Mode/Crop/Save/New buttons (100px), Threshold/Hue sliders
3. **Crop Phase**: Draggable crop handles, Cancel/Apply buttons (100px)

## Design Principles

### Touch-First UI:
- Large circular buttons (100-120px radius) with hover/press states
- Semi-transparent backgrounds (alpha 180-200) for modern look
- Triple spacing between sliders for fat-finger friendliness
- Bigger slider handles (0.8x width) with value bubbles on drag
- No cursor visible (force hidden in kiosk mode)
- 5-tap corner exit for touchscreen users

### Performance:
- 30 FPS camera streaming with frame buffering
- Immediate mode GUI (no retained state)
- Release builds with LTO and single codegen unit
- Texture reuse to avoid allocations
- Sleep mode after 5 minutes idle (dim logo, pause camera)

### User Experience:
- Splash screen (2s) with Harpy logo fade in/out on startup
- Auto-update check on launch from GitHub
- Session-based workflow (edit_001, edit_002, etc.)
- USB export when drive detected
- Export status popups (3s timeout)
- Sleep mode shows dim Harpy logo after 5 minutes idle

## File Structure

```
src/
  main.rs               Entry point, window config, touch styles
  ui.rs                 UI phases, buttons, sliders, rendering (1150+ lines)
  pixel_sorter.rs       Algorithms, hue/threshold processing
  camera_controller.rs  rpicam streaming, snapshot, test patterns
  session.rs            Save/iterate, USB export, directory management
  crop.rs               Crop rectangle logic, apply with sorting
  image_ops.rs          Image loading, tint blending
  texture.rs            egui texture helpers, frame optimization
  camera.rs             UI integration for camera capture

deployment/
  run_pixelsort.sh      Auto-update launcher (checks git, rebuilds)
  run_pixelsort.ps1     Windows version of launcher
  setup_autostart.sh    Install systemd service, enable autostart
  pixelsort-kiosk.service Systemd unit file
  README.md             Complete deployment guide

assets/
  Harpy_ICON.png        Harpy logo (splash screen + sleep mode)

sorted_images/          Output directory (git-ignored)
  session_YYYYMMDD_HHMMSS/
    edit_001_horizontal.png
    edit_002_vertical.png
    ...
```

## Development Guidelines

### Code Style:
- Use descriptive const names for UI sizing (`BUTTON_RADIUS`, `SLIDER_WIDTH`)
- Separate concerns: UI logic in ui.rs, algorithms in pixel_sorter.rs
- Avoid borrowing conflicts: clone `Arc` references when needed
- Prefer `ctx.request_repaint()` for animations over busy loops

### Adding Features:
- New UI elements: Add to appropriate phase in `ui.rs`
- New algorithms: Extend `SortingAlgorithm` enum in `pixel_sorter.rs`
- Camera changes: Modify `camera_controller.rs`, update streaming logic
- Deployment: Update scripts in `deployment/` folder

### Testing on Desktop:
- Camera unavailable → shows animated test pattern
- Camera functions work on Pi only
- UI fully functional for development
- Use `cargo run` for debug builds, `cargo run --release` for testing

### Cross-Compilation:
```bash
# Install cross
cargo install cross

# Build for Pi from Windows/Linux
cross build --release --target aarch64-unknown-linux-gnu

# Binary at: target/aarch64-unknown-linux-gnu/release/pixelsort-pi
```

## UI Constants (Easy Tweaks)

```rust
// src/ui.rs
const BUTTON_RADIUS: f32 = 100.0;       // Edit phase buttons
const LARGE_BUTTON_RADIUS: f32 = 120.0; // Take Picture button
const SMALL_BUTTON_RADIUS: f32 = 60.0;   // Upload button
const SLIDER_WIDTH: f32 = 80.0;          // Slider track width
const HANDLE_SIZE: f32 = 28.0;           // Crop handles

// src/main.rs
.with_inner_size([1920.0, 1080.0])  // Window resolution
ctx.set_zoom_factor(1.0);            // Disable DPI scaling

// Sleep timeout (src/ui.rs)
if idle_duration >= 300  // 300 seconds = 5 minutes
```

## Common Issues

### Cursor visible in buttons:
- Use `ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::None)` every frame

### Slider handles cut off:
- Add padding: `knob_radius` at top, `spacing * 5` at bottom

### Camera not streaming:
- Check `rpicam-vid` availability
- Verify camera cable connection
- Test with `rpicam-hello`

### Resolution wrong:
- Force `with_inner_size()` in ViewportBuilder
- Set `zoom_factor(1.0)` to disable scaling

### Git branch issues on Pi:
```bash
git checkout main
git branch -u origin/main
git reset --hard origin/main
```

## Release Checklist

- [ ] Update version in `Cargo.toml`
- [ ] Update `CHANGELOG.md` with new features
- [ ] Test on Raspberry Pi hardware
- [ ] Verify camera streaming works
- [ ] Test USB export functionality
- [ ] Ensure splash screen shows correctly
- [ ] Verify sleep mode triggers and wakes
- [ ] Test 5-tap exit mechanism
- [ ] Build release binary: `cargo build --release`
- [ ] Tag release: `git tag v0.x.x`
- [ ] Push tags: `git push --tags`

## Dependencies

Core:
- `eframe 0.24` - GUI framework
- `egui 0.24` - Immediate mode UI
- `image 0.24` - Image processing
- `tokio 1.0` - Async runtime (for camera)
- `anyhow 1.0` - Error handling
- `chrono 0.4` - Timestamps
- `rfd 0.11` - File dialogs

Platform-specific:
- `libgtk-3-dev` (Pi) - GUI backend
- `rpicam-vid`, `rpicam-still` (Pi) - Camera tools

## Architecture
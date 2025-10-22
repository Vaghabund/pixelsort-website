# Changelog

All notable changes to the Raspberry Pi Pixel Sorter project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Sleep mode: Activates after 5 minutes of inactivity, shows dim Harpy logo
- Auto-update on launch: Checks GitHub for updates before starting
- Kiosk mode: Fullscreen borderless window with hidden cursor
- Touch exit mechanism: Tap top-left corner 5 times within 3 seconds
- Circular touch UI: Large, touch-friendly circular buttons (100-120px radius)
- Semi-transparent UI elements: All buttons and sliders have alpha transparency
- Larger slider handles: Increased from 0.6x to 0.8x for better touch targets
- Deployment folder: Organized deployment scripts and documentation
- Comprehensive deployment guide with troubleshooting
- MIT License file

### Changed
- Button sizes increased: Edit phase buttons now 100px, input phase 120px/60px
- Slider spacing: Tripled horizontal spacing between sliders
- Force 1920x1080 resolution with zoom 1.0 (disable DPI scaling)
- Cursor hidden in all UI contexts
- Reorganized project structure for release readiness

### Fixed
- Cursor visibility in buttons and interactive elements
- Resolution scaling issues on 7" touchscreen
- Git branch tracking on Raspberry Pi

## [0.1.0] - 2025-01-XX

### Initial Release
- Horizontal, Vertical, Diagonal pixel sorting algorithms
- Live camera preview with rpicam integration
- Three-phase UI: Input → Edit → Crop
- Threshold and Hue sliders for control
- Save & Iterate workflow with session management
- USB export functionality
- Touch-optimized egui interface
- Cross-platform support (Windows/macOS/Linux/Pi)
- Rust implementation with eframe/egui

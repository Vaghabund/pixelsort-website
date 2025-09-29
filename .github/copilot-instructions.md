# Raspberry Pi Pixel Sorter Project

This is a manipulatable pixel sorting application designed for Raspberry Pi 5 with a 7-inch TFT screen and GPIO button controls.

## Project Overview
- **Target Platform**: Raspberry Pi 5
- **Display**: 7-inch TFT screen (HDMI)
- **Input**: GPIO buttons for algorithm manipulation
- **Language**: Python
- **GUI Framework**: tkinter (optimized for touchscreen)
- **Image Processing**: PIL/Pillow, NumPy
- **Hardware Control**: RPi.GPIO

## Key Features
- Real-time pixel sorting algorithms
- Interactive GUI optimized for 7-inch display
- GPIO button controls for parameter adjustment
- Multiple sorting algorithms (horizontal, vertical, diagonal)
- Image loading and saving capabilities
- Fullscreen mode for kiosk-style operation

## Development Guidelines
- Optimize UI elements for 7-inch screen (800x480 or 1024x600)
- Use large, touch-friendly buttons and controls
- Implement non-blocking pixel sorting for smooth interaction
- Follow Raspberry Pi GPIO best practices
- Design for standalone operation without keyboard/mouse

## File Structure
- `main.py` - Entry point and GUI setup
- `pixel_sorter.py` - Core pixel sorting algorithms
- `gpio_controller.py` - GPIO button handling
- `image_processor.py` - Image loading/saving utilities
- `config.py` - Configuration settings
- `requirements.txt` - Python dependencies
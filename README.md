# Raspberry Pi Pixel Sorter

A manipulatable pixel sorting application designed for Raspberry Pi 5 with a 7-inch TFT screen and GPIO button controls. Create stunning visual effects by sorting image pixels using various algorithms!

![Pixel Sorter Demo](demo_placeholder.gif)

## Features

- 🎨 **Multiple Sorting Algorithms**: Horizontal, vertical, diagonal, and radial pixel sorting
- 🖱️ **GPIO Button Controls**: Physical buttons for real-time parameter adjustment
- 📺 **Touch-Optimized UI**: Large buttons designed for 7-inch TFT screens
- ⚡ **Real-Time Processing**: Non-blocking algorithm execution with live preview
- 💾 **Image Management**: Load and save images in multiple formats
- 🔧 **Adjustable Parameters**: Fine-tune threshold and interval settings

## Hardware Requirements

- **Raspberry Pi 5** (recommended) or Raspberry Pi 4
- **7-inch TFT Display** connected via HDMI
- **5 GPIO Buttons** for interaction
- **MicroSD Card** (16GB+ recommended)
- **Power Supply** appropriate for your Pi model

## GPIO Button Wiring

Connect buttons to the following GPIO pins (BCM numbering):

| Button Function | GPIO Pin | Description |
|---|---|---|
| Load Image | 18 | Load new image from file system |
| Next Algorithm | 19 | Cycle through sorting algorithms |
| Threshold Up | 20 | Increase brightness threshold |
| Threshold Down | 21 | Decrease brightness threshold |
| Save Image | 26 | Save current sorted result |

### Wiring Diagram
```
Button -> GPIO Pin (with pull-up resistor)
         -> Ground
```

Each button should connect:
- One terminal to the specified GPIO pin
- Other terminal to Ground (GND)
- Use internal pull-up resistors (configured in software)

## Installation

### 1. Prepare Raspberry Pi

Update your system:
```bash
sudo apt update && sudo apt upgrade -y
```

Install Python dependencies:
```bash
sudo apt install python3-pip python3-tk python3-pil python3-numpy -y
```

### 2. Clone or Download Project

```bash
cd ~/
git clone <repository-url> pixelsort2
cd pixelsort2
```

Or download and extract the project files to `~/pixelsort2/`

### 3. Install Python Requirements

```bash
pip3 install -r requirements.txt
```

### 4. Configure Display (if needed)

For HDMI displays, you may need to edit `/boot/config.txt`:
```bash
sudo nano /boot/config.txt
```

Add/uncomment:
```
hdmi_force_hotplug=1
hdmi_drive=2
hdmi_mode=32  # For 1920x1080 displays
```

### 5. Test Installation

Run the application:
```bash
python3 main.py
```

## Usage

### Starting the Application

```bash
cd ~/pixelsort2
python3 main.py
```

The application will launch in fullscreen mode optimized for the 7-inch display.

### Using GPIO Buttons

1. **Load Image** (Button 1): Open file dialog to select an image
2. **Next Algorithm** (Button 2): Cycle through sorting algorithms:
   - Horizontal: Sorts pixels in horizontal lines
   - Vertical: Sorts pixels in vertical columns  
   - Diagonal: Sorts along diagonal lines
   - Radial: Sorts in radial patterns from center
3. **Threshold Up/Down** (Buttons 3/4): Adjust brightness threshold (0-255)
4. **Save Image** (Button 5): Save the current result

### Using Touch Interface

The GUI includes large, touch-friendly buttons that mirror the GPIO functionality:
- Load/Save buttons in the control panel
- Algorithm selection via radio buttons
- Threshold and interval sliders for fine adjustment

### Algorithm Parameters

- **Threshold**: Controls which pixels are grouped together for sorting (0-255)
  - Lower values: More aggressive sorting, larger groups
  - Higher values: More subtle effects, smaller groups
- **Interval**: Controls processing frequency
  - Lower values: Smoother gradients, slower processing
  - Higher values: More distinct bands, faster processing

## Configuration

Edit `config.py` to customize:

### Display Settings
```python
SCREEN_WIDTH = 800      # Your display width
SCREEN_HEIGHT = 480     # Your display height  
FULLSCREEN = True       # Fullscreen mode
```

### GPIO Pin Assignment
```python
GPIO_PINS = {
    'load_image': 18,
    'next_algorithm': 19,
    'threshold_up': 20,
    'threshold_down': 21,
    'save_image': 26
}
```

### Processing Parameters
```python
DEFAULT_THRESHOLD = 50
DEFAULT_INTERVAL = 10
```

## Development

### Running on Non-Pi Systems

For development on Windows/Mac/Linux without GPIO:
1. Install optional keyboard dependency: `pip install keyboard`
2. Run normally: `python main.py`
3. Use keyboard shortcuts: 1-5 keys for button functions

### Adding New Algorithms

1. Add algorithm function to `pixel_sorter.py`:
```python
def _sort_custom(self, img_array, **kwargs):
    # Your sorting logic here
    return modified_array
```

2. Register in the algorithms dictionary:
```python
self.algorithms['custom'] = self._sort_custom
```

### File Structure

```
pixelsort2/
├── main.py              # Main application entry point
├── pixel_sorter.py      # Pixel sorting algorithms
├── gpio_controller.py   # GPIO button handling
├── image_processor.py   # Image loading/saving utilities
├── config.py           # Configuration settings
├── requirements.txt    # Python dependencies
├── README.md          # This file
├── sample_images/     # Generated test images
└── .github/
    └── copilot-instructions.md  # Project context
```

## Troubleshooting

### Common Issues

**"No module named 'RPi.GPIO'"**
- Install GPIO library: `sudo apt install python3-rpi.gpio`
- Or install via pip: `pip3 install RPi.GPIO`

**Display not showing correctly**
- Check HDMI connection and display configuration
- Verify `config.py` display settings match your screen
- Try disabling fullscreen: Set `FULLSCREEN = False`

**Buttons not responding**
- Check wiring connections to GPIO pins
- Verify pin assignments in `config.py`
- Test with: `python gpio_controller.py`

**Image processing too slow**
- Reduce image size (< 1920x1080 recommended)
- Increase interval parameter for faster processing
- Close other applications to free memory

**Permission errors**
- Ensure user is in `gpio` group: `sudo usermod -a -G gpio $USER`
- Restart after adding to group

### Performance Tips

- Use images smaller than 1920x1080 for smooth real-time processing
- Adjust interval parameter to balance quality vs. speed
- Close unnecessary applications to free memory
- Use fast SD card (Class 10+ recommended)

## Auto-Start on Boot (Optional)

To launch the pixel sorter automatically on boot:

1. Create systemd service:
```bash
sudo nano /etc/systemd/system/pixelsort.service
```

2. Add service configuration:
```ini
[Unit]
Description=Pixel Sorter Application
After=graphical-session.target

[Service]
Type=simple
User=pi
Environment=DISPLAY=:0
WorkingDirectory=/home/pi/pixelsort2
ExecStart=/usr/bin/python3 main.py
Restart=always

[Install]
WantedBy=graphical-session.target
```

3. Enable service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable pixelsort.service
```

## License

This project is open source and available under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit issues and enhancement requests.

## Acknowledgments

- Inspired by pixel sorting art techniques
- Built for the Raspberry Pi community
- Thanks to the PIL/Pillow and NumPy teams for excellent image processing libraries
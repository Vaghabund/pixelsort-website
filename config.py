"""
Configuration settings for the Raspberry Pi Pixel Sorter application
"""

# Display settings for 7-inch TFT screen
SCREEN_WIDTH = 800
SCREEN_HEIGHT = 480
FULLSCREEN = True  # Set to False for development

# Image display area settings  
IMAGE_WIDTH = 480
IMAGE_HEIGHT = 360

# UI color scheme
BG_COLOR = '#2C3E50'
BUTTON_COLOR = '#3498DB'
TEXT_COLOR = '#ECF0F1'

# GPIO pin assignments (BCM numbering)
GPIO_PINS = {
    'load_image': 18,    # Button 1: Load new image
    'next_algorithm': 19, # Button 2: Cycle through algorithms  
    'threshold_up': 20,   # Button 3: Increase threshold
    'threshold_down': 21, # Button 4: Decrease threshold
    'save_image': 26      # Button 5: Save current result
}

# Pixel sorting default parameters
DEFAULT_THRESHOLD = 50
DEFAULT_INTERVAL = 10
MIN_THRESHOLD = 0
MAX_THRESHOLD = 255
MIN_INTERVAL = 1 
MAX_INTERVAL = 50

# Image processing settings
SUPPORTED_FORMATS = ['.jpg', '.jpeg', '.png', '.bmp', '.gif', '.tiff']
MAX_IMAGE_SIZE = (1920, 1080)  # Max resolution to prevent memory issues

# Default sample images directory
SAMPLE_IMAGES_DIR = 'sample_images'
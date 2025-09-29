#!/usr/bin/env python3
"""
Launch script for Raspberry Pi Pixel Sorter
Handles environment setup and graceful startup/shutdown
"""

import sys
import os
import signal
import argparse
from pathlib import Path

# Add current directory to Python path
sys.path.insert(0, str(Path(__file__).parent))

try:
    from main import main, PixelSorterApp
    import config
except ImportError as e:
    print(f"❌ Import error: {e}")
    print("Make sure all required dependencies are installed:")
    print("pip3 install -r requirements.txt")
    sys.exit(1)


def signal_handler(signum, frame):
    """Handle shutdown signals gracefully"""
    print("\n🔄 Shutting down pixel sorter...")
    sys.exit(0)


def check_environment():
    """Check if environment is properly configured"""
    issues = []
    
    # Check Python version
    if sys.version_info < (3, 7):
        issues.append("Python 3.7+ required")
    
    # Check required modules
    required_modules = ['PIL', 'numpy', 'tkinter']
    for module in required_modules:
        try:
            __import__(module)
        except ImportError:
            issues.append(f"Missing module: {module}")
    
    # Check GPIO (if on Raspberry Pi)
    try:
        import RPi.GPIO as GPIO
        GPIO.setmode(GPIO.BCM)
        GPIO.cleanup()
    except ImportError:
        print("ℹ️  RPi.GPIO not available - GPIO buttons will be simulated")
    except Exception as e:
        issues.append(f"GPIO error: {e}")
    
    # Check sample images
    if not os.path.exists(config.SAMPLE_IMAGES_DIR):
        print("📁 Creating sample images...")
        try:
            from image_processor import ImageProcessor
            processor = ImageProcessor()
            processor.create_sample_images()
        except Exception as e:
            issues.append(f"Could not create sample images: {e}")
    
    return issues


def main_with_args():
    """Main function with command line argument parsing"""
    parser = argparse.ArgumentParser(description='Raspberry Pi Pixel Sorter')
    parser.add_argument('--windowed', action='store_true', 
                       help='Run in windowed mode instead of fullscreen')
    parser.add_argument('--debug', action='store_true',
                       help='Enable debug output')
    parser.add_argument('--check', action='store_true',
                       help='Check environment and exit')
    
    args = parser.parse_args()
    
    # Set up signal handlers
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)
    
    # Check environment
    print("🔍 Checking environment...")
    issues = check_environment()
    
    if issues:
        print("❌ Environment issues found:")
        for issue in issues:
            print(f"   - {issue}")
        if not args.debug:
            return 1
    else:
        print("✅ Environment check passed")
    
    if args.check:
        return 0
    
    # Override fullscreen setting if windowed mode requested
    if args.windowed:
        config.FULLSCREEN = False
        print("🪟 Running in windowed mode")
    
    if args.debug:
        print("🐛 Debug mode enabled")
        import logging
        logging.basicConfig(level=logging.DEBUG)
    
    print("🎨 Starting Raspberry Pi Pixel Sorter...")
    print("   Press Ctrl+C to exit")
    
    try:
        main()
    except KeyboardInterrupt:
        print("\n👋 Goodbye!")
    except Exception as e:
        print(f"💥 Unexpected error: {e}")
        if args.debug:
            import traceback
            traceback.print_exc()
        return 1
    
    return 0


if __name__ == "__main__":
    sys.exit(main_with_args())
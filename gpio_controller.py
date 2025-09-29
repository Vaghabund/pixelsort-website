"""
GPIO Controller for Raspberry Pi button inputs
Handles hardware button presses for pixel sorter interaction
"""

import time
import threading
from config import GPIO_PINS

try:
    import RPi.GPIO as GPIO
    HAS_GPIO = True
except ImportError:
    # Fallback for development on non-Raspberry Pi systems
    HAS_GPIO = False
    print("Warning: RPi.GPIO not available. Using keyboard simulation.")


class GPIOController:
    def __init__(self, callback_function):
        """
        Initialize GPIO controller
        
        Args:
            callback_function: Function to call when button is pressed
                              Should accept button_id as parameter
        """
        self.callback = callback_function
        self.button_states = {}
        self.debounce_delay = 0.2  # 200ms debounce
        self.last_press_time = {}
        
        if HAS_GPIO:
            self._setup_gpio()
        else:
            self._setup_keyboard_simulation()
    
    def _setup_gpio(self):
        """Setup GPIO pins for button inputs"""
        GPIO.setmode(GPIO.BCM)  # Use BCM pin numbering
        GPIO.setwarnings(False)
        
        # Setup each button pin
        for button_name, pin in GPIO_PINS.items():
            GPIO.setup(pin, GPIO.IN, pull_up_down=GPIO.PUD_UP)
            
            # Add event detection with debounce
            GPIO.add_event_detect(
                pin, 
                GPIO.FALLING,  # Button press (assuming active low)
                callback=lambda channel, btn=button_name: self._button_callback(btn, channel),
                bouncetime=200
            )
            
            self.last_press_time[button_name] = 0
        
        print("GPIO setup complete. Button assignments:")
        for button_name, pin in GPIO_PINS.items():
            print(f"  {button_name}: GPIO {pin}")
    
    def _setup_keyboard_simulation(self):
        """Setup keyboard simulation for development"""
        print("Using keyboard simulation. Press these keys:")
        print("  1: Load Image")
        print("  2: Next Algorithm") 
        print("  3: Threshold Up")
        print("  4: Threshold Down")
        print("  5: Save Image")
        print("  ESC: Exit")
        
        # Start keyboard listener thread
        self.keyboard_thread = threading.Thread(target=self._keyboard_listener, daemon=True)
        self.keyboard_thread.start()
    
    def _keyboard_listener(self):
        """Simulate GPIO with keyboard input"""
        try:
            import keyboard
            
            keyboard.add_hotkey('1', lambda: self._simulate_button_press(1))
            keyboard.add_hotkey('2', lambda: self._simulate_button_press(2))
            keyboard.add_hotkey('3', lambda: self._simulate_button_press(3))
            keyboard.add_hotkey('4', lambda: self._simulate_button_press(4))
            keyboard.add_hotkey('5', lambda: self._simulate_button_press(5))
            
            keyboard.wait('esc')
            
        except ImportError:
            print("Install 'keyboard' package for keyboard simulation: pip install keyboard")
            # Fallback to basic input
            self._basic_input_simulation()
    
    def _basic_input_simulation(self):
        """Basic command line input simulation"""
        while True:
            try:
                key = input("Enter button (1-5) or 'q' to quit: ").strip()
                if key == 'q':
                    break
                elif key in ['1', '2', '3', '4', '5']:
                    self._simulate_button_press(int(key))
                else:
                    print("Invalid input. Use 1-5 or 'q'")
            except (KeyboardInterrupt, EOFError):
                break
    
    def _button_callback(self, button_name, channel):
        """Handle GPIO button press"""
        current_time = time.time()
        
        # Debounce check
        if current_time - self.last_press_time.get(button_name, 0) < self.debounce_delay:
            return
        
        self.last_press_time[button_name] = current_time
        
        # Map button name to button ID
        button_mapping = {
            'load_image': 1,
            'next_algorithm': 2,
            'threshold_up': 3,
            'threshold_down': 4,
            'save_image': 5
        }
        
        button_id = button_mapping.get(button_name)
        if button_id and self.callback:
            print(f"Button pressed: {button_name} (ID: {button_id})")
            self.callback(button_id)
    
    def _simulate_button_press(self, button_id):
        """Simulate a button press for development"""
        current_time = time.time()
        
        # Apply debounce to simulated presses too
        if current_time - self.last_press_time.get(button_id, 0) < self.debounce_delay:
            return
            
        self.last_press_time[button_id] = current_time
        
        button_names = {1: 'Load Image', 2: 'Next Algorithm', 3: 'Threshold Up', 
                       4: 'Threshold Down', 5: 'Save Image'}
        
        print(f"Simulated button press: {button_names.get(button_id, 'Unknown')} (ID: {button_id})")
        
        if self.callback:
            self.callback(button_id)
    
    def read_button_state(self, button_name):
        """Read current state of a button (for polling)"""
        if not HAS_GPIO:
            return False
            
        pin = GPIO_PINS.get(button_name)
        if pin is None:
            return False
            
        # Invert reading since we're using pull-up (button pressed = LOW)
        return not GPIO.input(pin)
    
    def get_all_button_states(self):
        """Get current state of all buttons"""
        states = {}
        for button_name in GPIO_PINS.keys():
            states[button_name] = self.read_button_state(button_name)
        return states
    
    def cleanup(self):
        """Clean up GPIO resources"""
        if HAS_GPIO:
            GPIO.cleanup()
            print("GPIO cleanup complete")
    
    def test_buttons(self, duration=10):
        """Test button functionality for specified duration (seconds)"""
        print(f"Testing buttons for {duration} seconds...")
        print("Press any button to test.")
        
        start_time = time.time()
        
        while time.time() - start_time < duration:
            if HAS_GPIO:
                # Show current button states
                states = self.get_all_button_states()
                pressed = [name for name, state in states.items() if state]
                if pressed:
                    print(f"Buttons currently pressed: {', '.join(pressed)}")
            
            time.sleep(0.1)
        
        print("Button test complete.")


class ButtonConfig:
    """Configuration helper for button assignments"""
    
    @staticmethod
    def get_button_descriptions():
        """Get human-readable button descriptions"""
        return {
            'load_image': 'Load new image from file system',
            'next_algorithm': 'Cycle through sorting algorithms',
            'threshold_up': 'Increase brightness threshold',
            'threshold_down': 'Decrease brightness threshold', 
            'save_image': 'Save current sorted image'
        }
    
    @staticmethod
    def validate_gpio_pins():
        """Validate that GPIO pins don't conflict"""
        pins = list(GPIO_PINS.values())
        if len(pins) != len(set(pins)):
            duplicates = [pin for pin in pins if pins.count(pin) > 1]
            raise ValueError(f"Duplicate GPIO pins detected: {duplicates}")
        
        # Check for reserved pins (add more as needed)
        reserved_pins = [0, 1, 14, 15]  # I2C, UART pins
        conflicts = [pin for pin in pins if pin in reserved_pins]
        if conflicts:
            print(f"Warning: Using reserved GPIO pins: {conflicts}")
        
        return True


# Development/testing entry point
if __name__ == "__main__":
    def test_callback(button_id):
        print(f"Button {button_id} pressed!")
    
    # Validate configuration
    ButtonConfig.validate_gpio_pins()
    
    # Test GPIO controller
    controller = GPIOController(test_callback)
    
    try:
        controller.test_buttons(30)  # Test for 30 seconds
    except KeyboardInterrupt:
        print("Test interrupted")
    finally:
        controller.cleanup()
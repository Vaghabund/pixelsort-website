#!/usr/bin/env python3
"""
Raspberry Pi Pixel Sorter - Main Application
A manipulatable pixel sorting application for Raspberry Pi 5 with 7-inch TFT screen.
"""

import tkinter as tk
from tkinter import ttk, filedialog, messagebox
import threading
from PIL import Image, ImageTk
import os
import sys

# Import our custom modules
from pixel_sorter import PixelSorter
from gpio_controller import GPIOController
from image_processor import ImageProcessor
from config import Config


class PixelSorterApp:
    def __init__(self, root):
        self.root = root
        self.setup_window()
        
        # Initialize components
        self.pixel_sorter = PixelSorter()
        self.image_processor = ImageProcessor()
        self.gpio_controller = GPIOController(self.on_button_press)
        
        # Application state
        self.current_image = None
        self.sorted_image = None
        self.processing = False
        
        self.create_widgets()
        self.setup_layout()
        
    def setup_window(self):
        """Configure the main window for 7-inch TFT display"""
        self.root.title("Raspberry Pi Pixel Sorter")
        
        # Optimize for 7-inch screen (800x480 typical resolution)
        self.root.geometry(f"{Config.SCREEN_WIDTH}x{Config.SCREEN_HEIGHT}")
        self.root.configure(bg=Config.BG_COLOR)
        
        # Fullscreen mode for kiosk operation
        if Config.FULLSCREEN:
            self.root.attributes('-fullscreen', True)
            
        # Make window not resizable
        self.root.resizable(False, False)
        
    def create_widgets(self):
        """Create GUI widgets optimized for touch interaction"""
        
        # Main frame
        self.main_frame = ttk.Frame(self.root)
        
        # Image display area
        self.image_frame = ttk.Frame(self.main_frame)
        self.canvas = tk.Canvas(
            self.image_frame, 
            width=Config.IMAGE_WIDTH, 
            height=Config.IMAGE_HEIGHT,
            bg='black'
        )
        
        # Control panel
        self.control_frame = ttk.Frame(self.main_frame)
        
        # Large touch-friendly buttons
        button_style = {
            'width': 15,
            'height': 2,
            'font': ('Arial', 12, 'bold')
        }
        
        self.load_btn = tk.Button(
            self.control_frame,
            text="Load Image",
            command=self.load_image,
            **button_style,
            bg=Config.BUTTON_COLOR
        )
        
        self.save_btn = tk.Button(
            self.control_frame,
            text="Save Result",
            command=self.save_image,
            **button_style,
            bg=Config.BUTTON_COLOR
        )
        
        # Algorithm selection
        self.algorithm_var = tk.StringVar(value="horizontal")
        algorithm_frame = ttk.Frame(self.control_frame)
        
        ttk.Label(algorithm_frame, text="Sort Algorithm:", font=('Arial', 10, 'bold')).pack()
        
        algorithms = [
            ("Horizontal", "horizontal"),
            ("Vertical", "vertical"), 
            ("Diagonal", "diagonal"),
            ("Radial", "radial")
        ]
        
        for text, value in algorithms:
            rb = ttk.Radiobutton(
                algorithm_frame,
                text=text,
                variable=self.algorithm_var,
                value=value,
                command=self.on_algorithm_change
            )
            rb.pack(anchor='w', pady=2)
        
        # Parameter controls
        self.create_parameter_controls()
        
        # Status display
        self.status_var = tk.StringVar(value="Ready - Load an image to begin")
        self.status_label = ttk.Label(
            self.control_frame,
            textvariable=self.status_var,
            font=('Arial', 10),
            foreground='blue'
        )
        
    def create_parameter_controls(self):
        """Create parameter adjustment controls"""
        param_frame = ttk.Frame(self.control_frame)
        
        # Threshold control
        ttk.Label(param_frame, text="Threshold:", font=('Arial', 10, 'bold')).grid(row=0, column=0, sticky='w')
        self.threshold_var = tk.DoubleVar(value=50.0)
        self.threshold_scale = tk.Scale(
            param_frame,
            from_=0,
            to=255,
            orient='horizontal',
            variable=self.threshold_var,
            command=self.on_parameter_change,
            length=200
        )
        self.threshold_scale.grid(row=0, column=1, padx=10)
        
        # Interval control  
        ttk.Label(param_frame, text="Interval:", font=('Arial', 10, 'bold')).grid(row=1, column=0, sticky='w')
        self.interval_var = tk.IntVar(value=10)
        self.interval_scale = tk.Scale(
            param_frame,
            from_=1,
            to=50,
            orient='horizontal',
            variable=self.interval_var,
            command=self.on_parameter_change,
            length=200
        )
        self.interval_scale.grid(row=1, column=1, padx=10)
        
        self.param_frame = param_frame
        
    def setup_layout(self):
        """Arrange widgets in the window"""
        self.main_frame.pack(fill='both', expand=True, padx=10, pady=10)
        
        # Left side - image display
        self.image_frame.pack(side='left', fill='both', expand=True)
        self.canvas.pack(padx=5, pady=5)
        
        # Right side - controls
        self.control_frame.pack(side='right', fill='y', padx=10)
        
        self.load_btn.pack(pady=10)
        self.save_btn.pack(pady=5)
        
        ttk.Separator(self.control_frame, orient='horizontal').pack(fill='x', pady=10)
        
        # Algorithm selection frame
        algorithm_frame = self.control_frame.nametowidget(
            [w for w in self.control_frame.winfo_children() if isinstance(w, ttk.Frame)][0]
        )
        algorithm_frame.pack(pady=10)
        
        ttk.Separator(self.control_frame, orient='horizontal').pack(fill='x', pady=10)
        
        self.param_frame.pack(pady=10)
        
        ttk.Separator(self.control_frame, orient='horizontal').pack(fill='x', pady=10)
        
        self.status_label.pack(pady=10)
        
    def load_image(self):
        """Load an image file"""
        file_path = filedialog.askopenfilename(
            title="Select Image",
            filetypes=[
                ("Image files", "*.jpg *.jpeg *.png *.bmp *.gif *.tiff"),
                ("All files", "*.*")
            ]
        )
        
        if file_path:
            try:
                self.current_image = self.image_processor.load_image(file_path)
                self.display_image(self.current_image)
                self.status_var.set(f"Loaded: {os.path.basename(file_path)}")
                self.auto_sort()
            except Exception as e:
                messagebox.showerror("Error", f"Failed to load image: {str(e)}")
                
    def save_image(self):
        """Save the sorted image"""
        if self.sorted_image is None:
            messagebox.showwarning("Warning", "No processed image to save")
            return
            
        file_path = filedialog.asksaveasfilename(
            title="Save Sorted Image",
            defaultextension=".png",
            filetypes=[
                ("PNG files", "*.png"),
                ("JPEG files", "*.jpg"),
                ("All files", "*.*")
            ]
        )
        
        if file_path:
            try:
                self.image_processor.save_image(self.sorted_image, file_path)
                self.status_var.set(f"Saved: {os.path.basename(file_path)}")
            except Exception as e:
                messagebox.showerror("Error", f"Failed to save image: {str(e)}")
                
    def display_image(self, pil_image):
        """Display PIL image on canvas"""
        # Resize image to fit canvas while maintaining aspect ratio
        display_image = self.image_processor.resize_for_display(
            pil_image, Config.IMAGE_WIDTH, Config.IMAGE_HEIGHT
        )
        
        # Convert to PhotoImage and display
        photo = ImageTk.PhotoImage(display_image)
        
        self.canvas.delete("all")
        self.canvas.create_image(
            Config.IMAGE_WIDTH // 2, 
            Config.IMAGE_HEIGHT // 2, 
            image=photo
        )
        
        # Keep a reference to prevent garbage collection
        self.canvas.image = photo
        
    def auto_sort(self):
        """Automatically apply pixel sorting with current parameters"""
        if self.current_image is None or self.processing:
            return
            
        self.processing = True
        self.status_var.set("Processing...")
        
        # Run pixel sorting in separate thread to keep UI responsive
        def sort_thread():
            try:
                params = {
                    'threshold': self.threshold_var.get(),
                    'interval': self.interval_var.get()
                }
                
                self.sorted_image = self.pixel_sorter.sort_pixels(
                    self.current_image.copy(),
                    self.algorithm_var.get(),
                    **params
                )
                
                # Update display in main thread
                self.root.after(0, lambda: self.display_image(self.sorted_image))
                self.root.after(0, lambda: self.status_var.set("Processing complete"))
                
            except Exception as e:
                self.root.after(0, lambda: messagebox.showerror("Error", f"Processing failed: {str(e)}"))
                self.root.after(0, lambda: self.status_var.set("Processing failed"))
            finally:
                self.processing = False
                
        threading.Thread(target=sort_thread, daemon=True).start()
        
    def on_algorithm_change(self):
        """Handle algorithm selection change"""
        self.auto_sort()
        
    def on_parameter_change(self, value):
        """Handle parameter adjustment"""
        # Debounce parameter changes to avoid excessive processing
        if hasattr(self, '_param_timer'):
            self.root.after_cancel(self._param_timer)
        self._param_timer = self.root.after(500, self.auto_sort)
        
    def on_button_press(self, button_id):
        """Handle GPIO button press"""
        if button_id == 1:  # Load image button
            self.load_image()
        elif button_id == 2:  # Next algorithm
            algorithms = ["horizontal", "vertical", "diagonal", "radial"]
            current_idx = algorithms.index(self.algorithm_var.get())
            next_idx = (current_idx + 1) % len(algorithms)
            self.algorithm_var.set(algorithms[next_idx])
            self.auto_sort()
        elif button_id == 3:  # Increase threshold
            current = self.threshold_var.get()
            self.threshold_var.set(min(255, current + 10))
            self.auto_sort()
        elif button_id == 4:  # Decrease threshold
            current = self.threshold_var.get()
            self.threshold_var.set(max(0, current - 10))
            self.auto_sort()
        elif button_id == 5:  # Save image
            self.save_image()
            
    def cleanup(self):
        """Clean up resources"""
        self.gpio_controller.cleanup()


def main():
    """Main application entry point"""
    root = tk.Tk()
    app = PixelSorterApp(root)
    
    try:
        root.mainloop()
    except KeyboardInterrupt:
        print("Application interrupted")
    finally:
        app.cleanup()


if __name__ == "__main__":
    main()
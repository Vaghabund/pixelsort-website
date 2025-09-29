"""
Image Processing Utilities
Handles image loading, saving, and display preparation for the pixel sorter
"""

import os
from PIL import Image, ImageTk
import numpy as np
from config import SUPPORTED_FORMATS, MAX_IMAGE_SIZE, SAMPLE_IMAGES_DIR


class ImageProcessor:
    def __init__(self):
        """Initialize image processor"""
        self.supported_formats = SUPPORTED_FORMATS
        self.max_size = MAX_IMAGE_SIZE
        
    def load_image(self, file_path):
        """
        Load an image from file path
        
        Args:
            file_path: Path to image file
            
        Returns:
            PIL Image object
            
        Raises:
            ValueError: If file format not supported
            FileNotFoundError: If file doesn't exist
        """
        if not os.path.exists(file_path):
            raise FileNotFoundError(f"Image file not found: {file_path}")
        
        # Check file extension
        file_ext = os.path.splitext(file_path)[1].lower()
        if file_ext not in self.supported_formats:
            raise ValueError(f"Unsupported file format: {file_ext}")
        
        try:
            # Load image
            image = Image.open(file_path)
            
            # Convert to RGB if necessary
            if image.mode != 'RGB':
                image = image.convert('RGB')
            
            # Resize if too large
            if image.size[0] > self.max_size[0] or image.size[1] > self.max_size[1]:
                print(f"Resizing large image from {image.size} to fit {self.max_size}")
                image.thumbnail(self.max_size, Image.Resampling.LANCZOS)
            
            return image
            
        except Exception as e:
            raise ValueError(f"Failed to load image: {str(e)}")
    
    def save_image(self, image, file_path, quality=95):
        """
        Save PIL image to file
        
        Args:
            image: PIL Image object
            file_path: Output file path
            quality: JPEG quality (0-100, ignored for PNG)
        """
        try:
            # Ensure output directory exists
            os.makedirs(os.path.dirname(file_path), exist_ok=True)
            
            # Get file extension to determine format
            file_ext = os.path.splitext(file_path)[1].lower()
            
            if file_ext in ['.jpg', '.jpeg']:
                image.save(file_path, 'JPEG', quality=quality, optimize=True)
            elif file_ext == '.png':
                image.save(file_path, 'PNG', optimize=True)
            else:
                # Default to PNG for other formats
                image.save(file_path, 'PNG')
                
        except Exception as e:
            raise ValueError(f"Failed to save image: {str(e)}")
    
    def resize_for_display(self, image, target_width, target_height):
        """
        Resize image to fit display area while maintaining aspect ratio
        
        Args:
            image: PIL Image object
            target_width: Target display width
            target_height: Target display height
            
        Returns:
            Resized PIL Image object
        """
        # Calculate aspect ratios
        img_ratio = image.size[0] / image.size[1]
        target_ratio = target_width / target_height
        
        if img_ratio > target_ratio:
            # Image is wider - fit to width
            new_width = target_width
            new_height = int(target_width / img_ratio)
        else:
            # Image is taller - fit to height
            new_height = target_height
            new_width = int(target_height * img_ratio)
        
        return image.resize((new_width, new_height), Image.Resampling.LANCZOS)
    
    def create_sample_images(self):
        """Create sample images for testing if none exist"""
        sample_dir = SAMPLE_IMAGES_DIR
        
        if not os.path.exists(sample_dir):
            os.makedirs(sample_dir)
        
        # Create gradient sample
        self._create_gradient_sample(os.path.join(sample_dir, "gradient.png"))
        
        # Create noise sample
        self._create_noise_sample(os.path.join(sample_dir, "noise.png"))
        
        # Create pattern sample
        self._create_pattern_sample(os.path.join(sample_dir, "pattern.png"))
        
        print(f"Sample images created in {sample_dir}")
    
    def _create_gradient_sample(self, file_path):
        """Create a gradient test image"""
        width, height = 400, 300
        image = Image.new('RGB', (width, height))
        pixels = []
        
        for y in range(height):
            for x in range(width):
                # Create horizontal gradient
                r = int(255 * x / width)
                g = int(255 * y / height)
                b = int(255 * (x + y) / (width + height))
                pixels.append((r, g, b))
        
        image.putdata(pixels)
        image.save(file_path)
    
    def _create_noise_sample(self, file_path):
        """Create a random noise test image"""
        width, height = 400, 300
        
        # Generate random noise
        noise_array = np.random.randint(0, 256, (height, width, 3), dtype=np.uint8)
        
        # Add some structure to make sorting more interesting
        for y in range(height):
            for x in range(width):
                if (x // 20 + y // 20) % 2 == 0:
                    noise_array[y, x] = [min(255, noise_array[y, x, i] + 50) for i in range(3)]
        
        image = Image.fromarray(noise_array)
        image.save(file_path)
    
    def _create_pattern_sample(self, file_path):
        """Create a geometric pattern test image"""
        width, height = 400, 300
        image = Image.new('RGB', (width, height))
        pixels = []
        
        for y in range(height):
            for x in range(width):
                # Create checkerboard with gradients
                checker_x = (x // 40) % 2
                checker_y = (y // 30) % 2
                
                if checker_x == checker_y:
                    # Light squares with gradient
                    r = 150 + int(105 * x / width)
                    g = 150 + int(105 * y / height)
                    b = 200
                else:
                    # Dark squares with gradient
                    r = int(100 * x / width)
                    g = int(100 * y / height)
                    b = 50
                
                pixels.append((r, g, b))
        
        image.putdata(pixels)
        image.save(file_path)
    
    def get_sample_images(self):
        """Get list of available sample images"""
        sample_dir = SAMPLE_IMAGES_DIR
        
        if not os.path.exists(sample_dir):
            self.create_sample_images()
        
        sample_files = []
        for file_name in os.listdir(sample_dir):
            file_ext = os.path.splitext(file_name)[1].lower()
            if file_ext in self.supported_formats:
                sample_files.append(os.path.join(sample_dir, file_name))
        
        return sorted(sample_files)
    
    def validate_image(self, image):
        """
        Validate that image is suitable for processing
        
        Args:
            image: PIL Image object
            
        Returns:
            tuple: (is_valid, error_message)
        """
        if not isinstance(image, Image.Image):
            return False, "Invalid image object"
        
        if image.size[0] < 10 or image.size[1] < 10:
            return False, "Image too small (minimum 10x10 pixels)"
        
        if image.size[0] > self.max_size[0] or image.size[1] > self.max_size[1]:
            return False, f"Image too large (maximum {self.max_size[0]}x{self.max_size[1]})"
        
        if image.mode not in ['RGB', 'RGBA', 'L']:
            return False, f"Unsupported image mode: {image.mode}"
        
        return True, "Valid"
    
    def get_image_info(self, image):
        """
        Get information about an image
        
        Args:
            image: PIL Image object
            
        Returns:
            dict: Image information
        """
        return {
            'size': image.size,
            'mode': image.mode,
            'format': getattr(image, 'format', 'Unknown'),
            'has_transparency': 'transparency' in image.info or image.mode == 'RGBA'
        }
    
    def create_thumbnail(self, image, size=(128, 128)):
        """Create a thumbnail of the image"""
        thumbnail = image.copy()
        thumbnail.thumbnail(size, Image.Resampling.LANCZOS)
        return thumbnail


# Testing and utility functions
if __name__ == "__main__":
    processor = ImageProcessor()
    
    # Create sample images for testing
    processor.create_sample_images()
    
    # Test loading sample images
    samples = processor.get_sample_images()
    print(f"Created {len(samples)} sample images:")
    
    for sample_path in samples:
        try:
            img = processor.load_image(sample_path)
            info = processor.get_image_info(img)
            is_valid, msg = processor.validate_image(img)
            
            print(f"  {os.path.basename(sample_path)}: {info['size']} {info['mode']} - {msg}")
            
        except Exception as e:
            print(f"  {os.path.basename(sample_path)}: ERROR - {e}")
"""
Pixel Sorting Algorithms
Various algorithms for manipulating and sorting pixels in images.
"""

import numpy as np
from PIL import Image
import math


class PixelSorter:
    def __init__(self):
        self.algorithms = {
            'horizontal': self._sort_horizontal,
            'vertical': self._sort_vertical,
            'diagonal': self._sort_diagonal,
            'radial': self._sort_radial
        }
    
    def sort_pixels(self, image, algorithm='horizontal', **kwargs):
        """
        Apply pixel sorting algorithm to image
        
        Args:
            image: PIL Image object
            algorithm: Algorithm name ('horizontal', 'vertical', 'diagonal', 'radial')
            **kwargs: Algorithm-specific parameters
        
        Returns:
            PIL Image object with sorted pixels
        """
        if algorithm not in self.algorithms:
            raise ValueError(f"Unknown algorithm: {algorithm}")
            
        # Convert to numpy array for processing
        img_array = np.array(image)
        
        # Apply the selected algorithm
        sorted_array = self.algorithms[algorithm](img_array, **kwargs)
        
        # Convert back to PIL Image
        return Image.fromarray(sorted_array.astype(np.uint8))
    
    def _sort_horizontal(self, img_array, threshold=50, interval=10, **kwargs):
        """Sort pixels horizontally within threshold ranges"""
        result = img_array.copy()
        height, width = img_array.shape[:2]
        
        for y in range(0, height, interval):
            for x in range(width):
                # Find sorting intervals based on threshold
                row = img_array[y, :]
                intervals = self._find_intervals(row, threshold)
                
                for start, end in intervals:
                    if end - start > 1:  # Only sort if interval has multiple pixels
                        # Sort by brightness
                        segment = result[y, start:end]
                        brightness = np.mean(segment, axis=1) if len(segment.shape) == 2 else segment
                        sorted_indices = np.argsort(brightness)
                        result[y, start:end] = segment[sorted_indices]
        
        return result
    
    def _sort_vertical(self, img_array, threshold=50, interval=10, **kwargs):
        """Sort pixels vertically within threshold ranges"""
        result = img_array.copy()
        height, width = img_array.shape[:2]
        
        for x in range(0, width, interval):
            for y in range(height):
                # Find sorting intervals based on threshold
                column = img_array[:, x]
                intervals = self._find_intervals(column, threshold)
                
                for start, end in intervals:
                    if end - start > 1:  # Only sort if interval has multiple pixels
                        # Sort by brightness
                        segment = result[start:end, x]
                        brightness = np.mean(segment, axis=1) if len(segment.shape) == 2 else segment
                        sorted_indices = np.argsort(brightness)
                        result[start:end, x] = segment[sorted_indices]
        
        return result
    
    def _sort_diagonal(self, img_array, threshold=50, interval=10, **kwargs):
        """Sort pixels along diagonal lines"""
        result = img_array.copy()
        height, width = img_array.shape[:2]
        
        # Sort along main diagonals
        for offset in range(-height, width, interval):
            diagonal_coords = []
            
            # Get diagonal coordinates
            if offset >= 0:
                # Upper diagonal
                for i in range(min(height, width - offset)):
                    diagonal_coords.append((i, i + offset))
            else:
                # Lower diagonal
                for i in range(min(width, height + offset)):
                    diagonal_coords.append((i - offset, i))
            
            if len(diagonal_coords) > 1:
                # Extract diagonal pixels
                diagonal_pixels = np.array([img_array[y, x] for y, x in diagonal_coords])
                intervals = self._find_intervals(diagonal_pixels, threshold)
                
                for start, end in intervals:
                    if end - start > 1:
                        segment = diagonal_pixels[start:end]
                        brightness = np.mean(segment, axis=1) if len(segment.shape) == 2 else segment
                        sorted_indices = np.argsort(brightness)
                        sorted_segment = segment[sorted_indices]
                        
                        # Put sorted pixels back
                        for i, idx in enumerate(range(start, end)):
                            y, x = diagonal_coords[idx]
                            result[y, x] = sorted_segment[i]
        
        return result
    
    def _sort_radial(self, img_array, threshold=50, interval=10, **kwargs):
        """Sort pixels in radial patterns from center"""
        result = img_array.copy()
        height, width = img_array.shape[:2]
        center_y, center_x = height // 2, width // 2
        
        # Create radial sorting lines
        num_lines = 36  # Number of radial lines
        for angle in range(0, 360, 360 // num_lines):
            if angle % (interval * 10) != 0:  # Skip some lines based on interval
                continue
                
            radians = math.radians(angle)
            line_coords = []
            
            # Trace line from center to edge
            max_radius = min(center_x, center_y, width - center_x, height - center_y)
            for r in range(1, max_radius):
                x = int(center_x + r * math.cos(radians))
                y = int(center_y + r * math.sin(radians))
                
                if 0 <= x < width and 0 <= y < height:
                    line_coords.append((y, x))
            
            if len(line_coords) > 1:
                # Extract line pixels
                line_pixels = np.array([img_array[y, x] for y, x in line_coords])
                intervals = self._find_intervals(line_pixels, threshold)
                
                for start, end in intervals:
                    if end - start > 1:
                        segment = line_pixels[start:end]
                        brightness = np.mean(segment, axis=1) if len(segment.shape) == 2 else segment
                        sorted_indices = np.argsort(brightness)
                        sorted_segment = segment[sorted_indices]
                        
                        # Put sorted pixels back
                        for i, idx in enumerate(range(start, end)):
                            y, x = line_coords[idx]
                            result[y, x] = sorted_segment[i]
        
        return result
    
    def _find_intervals(self, pixel_array, threshold):
        """
        Find intervals of pixels that should be sorted together
        based on brightness threshold
        """
        if len(pixel_array.shape) > 1:
            # Color image - use average brightness
            brightness = np.mean(pixel_array, axis=1)
        else:
            # Grayscale image
            brightness = pixel_array
        
        intervals = []
        start = 0
        
        for i in range(1, len(brightness)):
            # Check if brightness difference exceeds threshold
            if abs(brightness[i] - brightness[i-1]) > threshold:
                if i - start > 1:  # Only add intervals with multiple pixels
                    intervals.append((start, i))
                start = i
        
        # Add final interval
        if len(brightness) - start > 1:
            intervals.append((start, len(brightness)))
        
        return intervals
    
    def get_available_algorithms(self):
        """Return list of available sorting algorithms"""
        return list(self.algorithms.keys())
    
    def preview_sort(self, image, algorithm, **kwargs):
        """
        Generate a quick preview of the sorting effect
        (processes only every nth row/column for speed)
        """
        # Reduce processing for preview
        preview_kwargs = kwargs.copy()
        preview_kwargs['interval'] = kwargs.get('interval', 10) * 3
        
        return self.sort_pixels(image, algorithm, **preview_kwargs)
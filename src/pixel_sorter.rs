use anyhow::Result;
use image::{ImageBuffer, Rgb, RgbImage};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy)]
pub enum SortingAlgorithm {
    Horizontal,
    Vertical,
    Diagonal,
    Radial,
}

impl SortingAlgorithm {
    pub fn all() -> &'static [SortingAlgorithm] {
        &[
            SortingAlgorithm::Horizontal,
            SortingAlgorithm::Vertical,
            SortingAlgorithm::Diagonal,
            SortingAlgorithm::Radial,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            SortingAlgorithm::Horizontal => "Horizontal",
            SortingAlgorithm::Vertical => "Vertical",
            SortingAlgorithm::Diagonal => "Diagonal",
            SortingAlgorithm::Radial => "Radial",
        }
    }

    pub fn next(&self) -> SortingAlgorithm {
        let all = Self::all();
        let current_index = all.iter().position(|&x| std::mem::discriminant(&x) == std::mem::discriminant(self)).unwrap();
        all[(current_index + 1) % all.len()]
    }
}

#[derive(Debug, Clone)]
pub struct SortingParameters {
    pub threshold: f32,
    pub interval: usize,
}

impl Default for SortingParameters {
    fn default() -> Self {
        Self {
            threshold: 50.0,
            interval: 10,
        }
    }
}

pub struct PixelSorter;

impl PixelSorter {
    pub fn new() -> Self {
        Self
    }

    pub fn sort_pixels(
        &self,
        image: &RgbImage,
        algorithm: SortingAlgorithm,
        params: &SortingParameters,
    ) -> Result<RgbImage> {
        let (width, height) = image.dimensions();
        let mut result = image.clone();

        match algorithm {
            SortingAlgorithm::Horizontal => self.sort_horizontal(&mut result, params),
            SortingAlgorithm::Vertical => self.sort_vertical(&mut result, params),
            SortingAlgorithm::Diagonal => self.sort_diagonal(&mut result, params),
            SortingAlgorithm::Radial => self.sort_radial(&mut result, params),
        }

        Ok(result)
    }

    fn sort_horizontal(&self, image: &mut RgbImage, params: &SortingParameters) {
        let (width, height) = image.dimensions();
        
        for y in (0..height).step_by(params.interval) {
            let mut row_pixels: Vec<(usize, Rgb<u8>)> = (0..width)
                .map(|x| (x as usize, *image.get_pixel(x, y)))
                .collect();

            let intervals = self.find_intervals(&row_pixels, params.threshold);
            
            for (start, end) in intervals {
                if end - start > 1 {
                    let mut segment: Vec<_> = row_pixels[start..end].iter().map(|(_, pixel)| *pixel).collect();
                    segment.sort_by(|a, b| self.pixel_brightness(a).partial_cmp(&self.pixel_brightness(b)).unwrap_or(Ordering::Equal));
                    
                    for (i, &pixel) in segment.iter().enumerate() {
                        image.put_pixel((start + i) as u32, y, pixel);
                    }
                }
            }
        }
    }

    fn sort_vertical(&self, image: &mut RgbImage, params: &SortingParameters) {
        let (width, height) = image.dimensions();
        
        for x in (0..width).step_by(params.interval) {
            let mut col_pixels: Vec<(usize, Rgb<u8>)> = (0..height)
                .map(|y| (y as usize, *image.get_pixel(x, y)))
                .collect();

            let intervals = self.find_intervals(&col_pixels, params.threshold);
            
            for (start, end) in intervals {
                if end - start > 1 {
                    let mut segment: Vec<_> = col_pixels[start..end].iter().map(|(_, pixel)| *pixel).collect();
                    segment.sort_by(|a, b| self.pixel_brightness(a).partial_cmp(&self.pixel_brightness(b)).unwrap_or(Ordering::Equal));
                    
                    for (i, &pixel) in segment.iter().enumerate() {
                        image.put_pixel(x, (start + i) as u32, pixel);
                    }
                }
            }
        }
    }

    fn sort_diagonal(&self, image: &mut RgbImage, params: &SortingParameters) {
        let (width, height) = image.dimensions();
        let (w, h) = (width as i32, height as i32);
        
        // Sort main diagonals
        for offset in (-h..w).step_by(params.interval) {
            let mut diagonal_pixels = Vec::new();
            
            if offset >= 0 {
                // Upper diagonals
                for i in 0..std::cmp::min(h, w - offset) {
                    let x = (i + offset) as u32;
                    let y = i as u32;
                    diagonal_pixels.push(((x, y), *image.get_pixel(x, y)));
                }
            } else {
                // Lower diagonals
                for i in 0..std::cmp::min(w, h + offset) {
                    let x = i as u32;
                    let y = (i - offset) as u32;
                    diagonal_pixels.push(((x, y), *image.get_pixel(x, y)));
                }
            }

            if diagonal_pixels.len() <= 1 {
                continue;
            }

            let pixel_values: Vec<_> = diagonal_pixels.iter().map(|(_, pixel)| *pixel).collect();
            let intervals = self.find_intervals_from_pixels(&pixel_values, params.threshold);
            
            for (start, end) in intervals {
                if end - start > 1 {
                    let mut segment: Vec<_> = pixel_values[start..end].to_vec();
                    segment.sort_by(|a, b| self.pixel_brightness(a).partial_cmp(&self.pixel_brightness(b)).unwrap_or(Ordering::Equal));
                    
                    for (i, &pixel) in segment.iter().enumerate() {
                        let ((x, y), _) = diagonal_pixels[start + i];
                        image.put_pixel(x, y, pixel);
                    }
                }
            }
        }
    }

    fn sort_radial(&self, image: &mut RgbImage, params: &SortingParameters) {
        let (width, height) = image.dimensions();
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let max_radius = (center_x.min(center_y)) as usize;
        
        // Create radial lines
        let num_lines = 36;
        for angle_step in (0..num_lines).step_by(params.interval.max(1)) {
            let angle = (angle_step as f32 * 360.0 / num_lines as f32).to_radians();
            let mut line_pixels = Vec::new();
            
            for r in 1..max_radius {
                let x = (center_x + r as f32 * angle.cos()) as u32;
                let y = (center_y + r as f32 * angle.sin()) as u32;
                
                if x < width && y < height {
                    line_pixels.push(((x, y), *image.get_pixel(x, y)));
                }
            }

            if line_pixels.len() <= 1 {
                continue;
            }

            let pixel_values: Vec<_> = line_pixels.iter().map(|(_, pixel)| *pixel).collect();
            let intervals = self.find_intervals_from_pixels(&pixel_values, params.threshold);
            
            for (start, end) in intervals {
                if end - start > 1 {
                    let mut segment: Vec<_> = pixel_values[start..end].to_vec();
                    segment.sort_by(|a, b| self.pixel_brightness(a).partial_cmp(&self.pixel_brightness(b)).unwrap_or(Ordering::Equal));
                    
                    for (i, &pixel) in segment.iter().enumerate() {
                        let ((x, y), _) = line_pixels[start + i];
                        image.put_pixel(x, y, pixel);
                    }
                }
            }
        }
    }

    fn find_intervals(&self, pixels: &[(usize, Rgb<u8>)], threshold: f32) -> Vec<(usize, usize)> {
        let pixel_values: Vec<_> = pixels.iter().map(|(_, pixel)| *pixel).collect();
        self.find_intervals_from_pixels(&pixel_values, threshold)
    }

    fn find_intervals_from_pixels(&self, pixels: &[Rgb<u8>], threshold: f32) -> Vec<(usize, usize)> {
        if pixels.len() <= 1 {
            return Vec::new();
        }

        let mut intervals = Vec::new();
        let mut start = 0;

        for i in 1..pixels.len() {
            let brightness_diff = (self.pixel_brightness(&pixels[i]) - self.pixel_brightness(&pixels[i - 1])).abs();
            
            if brightness_diff > threshold {
                if i - start > 1 {
                    intervals.push((start, i));
                }
                start = i;
            }
        }

        // Add final interval
        if pixels.len() - start > 1 {
            intervals.push((start, pixels.len()));
        }

        intervals
    }

    fn pixel_brightness(&self, pixel: &Rgb<u8>) -> f32 {
        // Calculate luminance using standard RGB to grayscale conversion
        let r = pixel[0] as f32;
        let g = pixel[1] as f32;
        let b = pixel[2] as f32;
        
        0.299 * r + 0.587 * g + 0.114 * b
    }

    pub fn preview_sort(
        &self,
        image: &RgbImage,
        algorithm: SortingAlgorithm,
        params: &SortingParameters,
    ) -> Result<RgbImage> {
        // Create a faster preview by processing at lower resolution
        let (width, height) = image.dimensions();
        let scale_factor = 4; // Process every 4th pixel
        
        let preview_params = SortingParameters {
            threshold: params.threshold,
            interval: params.interval * scale_factor,
        };
        
        self.sort_pixels(image, algorithm, &preview_params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn test_pixel_brightness() {
        let sorter = PixelSorter::new();
        let white = Rgb([255, 255, 255]);
        let black = Rgb([0, 0, 0]);
        let red = Rgb([255, 0, 0]);
        
        assert!(sorter.pixel_brightness(&white) > sorter.pixel_brightness(&black));
        assert!(sorter.pixel_brightness(&white) > sorter.pixel_brightness(&red));
    }

    #[test]
    fn test_algorithm_cycling() {
        let algorithm = SortingAlgorithm::Horizontal;
        let next = algorithm.next();
        assert_eq!(next.name(), "Vertical");
    }

    #[test]
    fn test_sorting_parameters_default() {
        let params = SortingParameters::default();
        assert_eq!(params.threshold, 50.0);
        assert_eq!(params.interval, 10);
    }
}
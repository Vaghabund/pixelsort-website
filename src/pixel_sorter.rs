#![allow(dead_code)]
use anyhow::Result;
use image::{Rgb, RgbImage};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortingAlgorithm {
    Horizontal,
    Vertical,
    Diagonal,
}

impl std::fmt::Display for SortingAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl SortingAlgorithm {
    pub fn all() -> &'static [SortingAlgorithm] {
        &[
            SortingAlgorithm::Horizontal,
            SortingAlgorithm::Vertical,
            SortingAlgorithm::Diagonal,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            SortingAlgorithm::Horizontal => "Horizontal",
            SortingAlgorithm::Vertical => "Vertical",
            SortingAlgorithm::Diagonal => "Diagonal",
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
    pub hue_shift: f32,
}

impl Default for SortingParameters {
    fn default() -> Self {
        Self {
            threshold: 50.0,
            hue_shift: 0.0,
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
        let (_width, _height) = image.dimensions();
        let mut result = image.clone();

        // Apply hue shift first if needed
        if params.hue_shift != 0.0 {
            self.apply_hue_shift(&mut result, params.hue_shift);
        }

        match algorithm {
            SortingAlgorithm::Horizontal => self.sort_horizontal(&mut result, params),
            SortingAlgorithm::Vertical => self.sort_vertical(&mut result, params),
            SortingAlgorithm::Diagonal => self.sort_diagonal(&mut result, params),
        }

        Ok(result)
    }

    fn sort_horizontal(&self, image: &mut RgbImage, params: &SortingParameters) {
        let (width, height) = image.dimensions();
        
        for y in 0..height {
            let row_pixels: Vec<(usize, Rgb<u8>)> = (0..width)
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
        
        for x in 0..width {
            let col_pixels: Vec<(usize, Rgb<u8>)> = (0..height)
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
        for offset in -h..w {
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
        let (_width, _height) = image.dimensions();
        
        let preview_params = SortingParameters {
            threshold: params.threshold,
            hue_shift: params.hue_shift,
        };
        
        self.sort_pixels(image, algorithm, &preview_params)
    }

    fn apply_hue_shift(&self, image: &mut RgbImage, hue_shift: f32) {
        let (width, height) = image.dimensions();
        
        for y in 0..height {
            for x in 0..width {
                let pixel = image.get_pixel(x, y);
                let shifted_pixel = self.shift_pixel_hue(pixel, hue_shift);
                image.put_pixel(x, y, shifted_pixel);
            }
        }
    }

    fn shift_pixel_hue(&self, pixel: &Rgb<u8>, hue_shift: f32) -> Rgb<u8> {
        // Convert RGB to HSV
        let r = pixel[0] as f32 / 255.0;
        let g = pixel[1] as f32 / 255.0;
        let b = pixel[2] as f32 / 255.0;

        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let delta = max - min;

        let mut h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        if h < 0.0 {
            h += 360.0;
        }

        let s = if max == 0.0 { 0.0 } else { delta / max };
        let v = max;

        // Apply hue shift
        h = (h + hue_shift) % 360.0;
        if h < 0.0 {
            h += 360.0;
        }

        // Convert HSV back to RGB
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r_prime, g_prime, b_prime) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        let r_final = ((r_prime + m) * 255.0).round() as u8;
        let g_final = ((g_prime + m) * 255.0).round() as u8;
        let b_final = ((b_prime + m) * 255.0).round() as u8;

        Rgb([r_final, g_final, b_final])
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
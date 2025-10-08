#![allow(dead_code)]
use anyhow::{Context, Result};
use image::{ImageBuffer, Rgb, RgbImage};
use std::path::{Path, PathBuf};
use log::{info, debug};

pub struct ImageProcessor {
    supported_formats: Vec<&'static str>,
    max_dimensions: (u32, u32),
}

impl ImageProcessor {
    pub fn new() -> Self {
        Self {
            supported_formats: vec!["png", "jpg", "jpeg", "bmp", "gif", "tiff", "webp"],
            max_dimensions: (1920, 1080), // Maximum size to prevent memory issues
        }
    }

    pub fn load_image<P: AsRef<Path>>(&self, path: P) -> Result<RgbImage> {
        let path = path.as_ref();
        
        // Check if file exists
        if !path.exists() {
            return Err(anyhow::anyhow!("Image file not found: {}", path.display()));
        }

        // Check file extension
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            let ext_lower = extension.to_lowercase();
            if !self.supported_formats.contains(&ext_lower.as_str()) {
                return Err(anyhow::anyhow!("Unsupported file format: {}", extension));
            }
        } else {
            return Err(anyhow::anyhow!("No file extension found"));
        }

        // Load and process image
        let img = image::open(path)
            .with_context(|| format!("Failed to load image from {}", path.display()))?;

        let mut rgb_img = img.to_rgb8();

        // Resize if too large
        if rgb_img.width() > self.max_dimensions.0 || rgb_img.height() > self.max_dimensions.1 {
            debug!("Resizing large image from {}x{} to fit within {}x{}", 
                rgb_img.width(), rgb_img.height(), 
                self.max_dimensions.0, self.max_dimensions.1);
            
            rgb_img = self.resize_to_fit(&rgb_img, self.max_dimensions.0, self.max_dimensions.1);
        }

    debug!("Successfully loaded image: {}x{}", rgb_img.width(), rgb_img.height());
        Ok(rgb_img)
    }

    pub fn save_image<P: AsRef<Path>>(&self, image: &RgbImage, path: P) -> Result<()> {
        let path = path.as_ref();
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        // Save image
        image.save(path)
            .with_context(|| format!("Failed to save image to {}", path.display()))?;

    debug!("Image saved successfully to {}", path.display());
        Ok(())
    }

    pub fn resize_to_fit(&self, image: &RgbImage, max_width: u32, max_height: u32) -> RgbImage {
        let (width, height) = (image.width(), image.height());
        
        // Calculate new dimensions maintaining aspect ratio
        let width_ratio = max_width as f32 / width as f32;
        let height_ratio = max_height as f32 / height as f32;
        let scale = width_ratio.min(height_ratio);

        if scale >= 1.0 {
            return image.clone(); // No resizing needed
        }

        let new_width = (width as f32 * scale) as u32;
        let new_height = (height as f32 * scale) as u32;

        image::imageops::resize(image, new_width, new_height, image::imageops::FilterType::Lanczos3)
    }

    pub fn resize_for_display(&self, image: &RgbImage, display_width: u32, display_height: u32) -> RgbImage {
        self.resize_to_fit(image, display_width, display_height)
    }

    pub fn create_sample_images(&self, output_dir: &Path) -> Result<Vec<PathBuf>> {
        std::fs::create_dir_all(output_dir)
            .with_context(|| format!("Failed to create sample images directory: {}", output_dir.display()))?;

        let mut created_files = Vec::new();

        // Create gradient sample
        let gradient_path = output_dir.join("gradient_sample.png");
        let gradient_img = self.create_gradient_image(400, 300);
        gradient_img.save(&gradient_path)?;
        created_files.push(gradient_path);
    debug!("Created gradient sample image");

        // Create noise sample
        let noise_path = output_dir.join("noise_sample.png");
        let noise_img = self.create_noise_image(400, 300);
        noise_img.save(&noise_path)?;
        created_files.push(noise_path);
    debug!("Created noise sample image");

        // Create geometric pattern sample
        let pattern_path = output_dir.join("pattern_sample.png");
        let pattern_img = self.create_pattern_image(400, 300);
        pattern_img.save(&pattern_path)?;
        created_files.push(pattern_path);
    debug!("Created pattern sample image");

        // Create color bands sample
        let bands_path = output_dir.join("bands_sample.png");
        let bands_img = self.create_color_bands_image(400, 300);
        bands_img.save(&bands_path)?;
        created_files.push(bands_path);
    debug!("Created color bands sample image");

    debug!("Created {} sample images in {}", created_files.len(), output_dir.display());
        Ok(created_files)
    }

    fn create_gradient_image(&self, width: u32, height: u32) -> RgbImage {
        ImageBuffer::from_fn(width, height, |x, y| {
            let r = (255.0 * x as f32 / width as f32) as u8;
            let g = (255.0 * y as f32 / height as f32) as u8;
            let b = (255.0 * (x + y) as f32 / (width + height) as f32) as u8;
            Rgb([r, g, b])
        })
    }

    fn create_noise_image(&self, width: u32, height: u32) -> RgbImage {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        ImageBuffer::from_fn(width, height, |x, y| {
            // Pseudo-random noise generation
            let mut hasher = DefaultHasher::new();
            (x, y).hash(&mut hasher);
            let hash = hasher.finish();
            
            let r = (hash & 0xFF) as u8;
            let g = ((hash >> 8) & 0xFF) as u8;
            let b = ((hash >> 16) & 0xFF) as u8;
            
            // Add some structure to make sorting more interesting
            let structure_factor = if ((x / 20) + (y / 20)) % 2 == 0 { 50 } else { 0 };
            
            Rgb([
                r.saturating_add(structure_factor),
                g.saturating_add(structure_factor),
                b.saturating_add(structure_factor),
            ])
        })
    }

    fn create_pattern_image(&self, width: u32, height: u32) -> RgbImage {
        ImageBuffer::from_fn(width, height, |x, y| {
            let checker_x = (x / 40) % 2;
            let checker_y = (y / 30) % 2;
            
            if checker_x == checker_y {
                // Light squares with gradient
                let r = 150 + (105 * x / width) as u8;
                let g = 150 + (105 * y / height) as u8;
                let b = 200;
                Rgb([r, g, b])
            } else {
                // Dark squares with gradient
                let r = (100 * x / width) as u8;
                let g = (100 * y / height) as u8;
                let b = 50;
                Rgb([r, g, b])
            }
        })
    }

    fn create_color_bands_image(&self, width: u32, height: u32) -> RgbImage {
        ImageBuffer::from_fn(width, height, |x, _y| {
            let band_width = width / 6;
            let band_index = x / band_width;
            
            match band_index {
                0 => Rgb([255, 0, 0]),     // Red
                1 => Rgb([255, 165, 0]),   // Orange  
                2 => Rgb([255, 255, 0]),   // Yellow
                3 => Rgb([0, 255, 0]),     // Green
                4 => Rgb([0, 0, 255]),     // Blue
                _ => Rgb([128, 0, 128]),   // Purple
            }
        })
    }

    pub fn get_image_info(&self, image: &RgbImage) -> ImageInfo {
        let (width, height) = image.dimensions();
        let pixel_count = (width * height) as usize;
        
        // Calculate basic statistics
        let mut total_r = 0u64;
        let mut total_g = 0u64;
        let mut total_b = 0u64;
        
        for pixel in image.pixels() {
            total_r += pixel[0] as u64;
            total_g += pixel[1] as u64;
            total_b += pixel[2] as u64;
        }
        
        ImageInfo {
            width,
            height,
            pixel_count,
            average_color: Rgb([
                (total_r / pixel_count as u64) as u8,
                (total_g / pixel_count as u64) as u8,
                (total_b / pixel_count as u64) as u8,
            ]),
            file_size_estimate: pixel_count * 3, // RGB = 3 bytes per pixel
        }
    }

    pub fn validate_image(&self, image: &RgbImage) -> Result<()> {
        let (width, height) = image.dimensions();
        
        if width == 0 || height == 0 {
            return Err(anyhow::anyhow!("Invalid image dimensions: {}x{}", width, height));
        }
        
        if width > self.max_dimensions.0 || height > self.max_dimensions.1 {
            return Err(anyhow::anyhow!(
                "Image too large: {}x{} (max: {}x{})", 
                width, height, 
                self.max_dimensions.0, self.max_dimensions.1
            ));
        }
        
        Ok(())
    }

    pub fn create_thumbnail(&self, image: &RgbImage, max_size: u32) -> RgbImage {
        self.resize_to_fit(image, max_size, max_size)
    }

    pub fn supported_formats(&self) -> &[&str] {
        &self.supported_formats
    }

    pub fn max_dimensions(&self) -> (u32, u32) {
        self.max_dimensions
    }

    pub fn set_max_dimensions(&mut self, width: u32, height: u32) {
        self.max_dimensions = (width, height);
    debug!("Updated max image dimensions to {}x{}", width, height);
    }
}

#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub pixel_count: usize,
    pub average_color: Rgb<u8>,
    pub file_size_estimate: usize,
}

impl ImageInfo {
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
    
    pub fn megapixels(&self) -> f32 {
        self.pixel_count as f32 / 1_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_image_processor_creation() {
        let processor = ImageProcessor::new();
        assert!(!processor.supported_formats().is_empty());
        assert!(processor.max_dimensions().0 > 0);
        assert!(processor.max_dimensions().1 > 0);
    }

    #[test]
    fn test_gradient_image_creation() {
        let processor = ImageProcessor::new();
        let image = processor.create_gradient_image(100, 100);
        assert_eq!(image.dimensions(), (100, 100));
        
        // Check corners have expected colors
        let top_left = image.get_pixel(0, 0);
        let bottom_right = image.get_pixel(99, 99);
        assert!(top_left[0] < bottom_right[0]); // Red increases left to right
        assert!(top_left[1] < bottom_right[1]); // Green increases top to bottom
    }

    #[test]
    fn test_image_info() {
        let processor = ImageProcessor::new();
        let image = processor.create_gradient_image(10, 10);
        let info = processor.get_image_info(&image);
        
        assert_eq!(info.width, 10);
        assert_eq!(info.height, 10);
        assert_eq!(info.pixel_count, 100);
        assert_eq!(info.aspect_ratio(), 1.0);
    }

    #[test] 
    fn test_resize_to_fit() {
        let processor = ImageProcessor::new();
        let large_image = processor.create_gradient_image(2000, 1000);
        let resized = processor.resize_to_fit(&large_image, 800, 600);
        
        // Should be resized to fit within bounds while maintaining aspect ratio
        assert!(resized.width() <= 800);
        assert!(resized.height() <= 600);
        
        // Aspect ratio should be preserved
        let original_ratio = 2000.0 / 1000.0;
        let resized_ratio = resized.width() as f32 / resized.height() as f32;
        assert!((original_ratio - resized_ratio).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_sample_image_creation() {
        let processor = ImageProcessor::new();
        let temp_dir = TempDir::new().unwrap();
        
        let created_files = processor.create_sample_images(temp_dir.path()).unwrap();
        assert!(!created_files.is_empty());
        
        // Verify files were actually created
        for file in &created_files {
            assert!(file.exists());
        }
    }
}
use anyhow::{anyhow, Result};
use image::{RgbImage, ImageBuffer};
use std::path::Path;
use std::process::Command;
use tokio::time::{Duration, sleep};
use tokio::fs;

/// Camera controller for Raspberry Pi Camera v1.5 using libcamera
pub struct CameraController {
    /// Camera settings
    width: u32,
    height: u32,
    quality: u8,
    /// Temporary file path for captured images
    temp_image_path: String,
    /// Whether libcamera-still is available
    is_available: bool,
}

impl CameraController {
    /// Create a new camera controller
    pub fn new() -> Result<Self> {
        let mut controller = CameraController {
            width: 1024,  // Default resolution
            height: 768,
            quality: 85,  // JPEG quality (0-100)
            temp_image_path: "/tmp/pixelsort_camera_capture.jpg".to_string(),
            is_available: false,
        };

        controller.initialize()?;
        Ok(controller)
    }

    /// Initialize the camera by checking if libcamera-still is available
    pub fn initialize(&mut self) -> Result<()> {
        // Check if libcamera-still command is available
        match Command::new("libcamera-still").arg("--help").output() {
            Ok(_) => {
                self.is_available = true;
                log::info!("Raspberry Pi Camera initialized successfully (using libcamera-still)");
                Ok(())
            }
            Err(_) => {
                // Try legacy raspistill as fallback
                match Command::new("raspistill").arg("-?").output() {
                    Ok(_) => {
                        self.is_available = true;
                        log::info!("Raspberry Pi Camera initialized successfully (using legacy raspistill)");
                        Ok(())
                    }
                    Err(e) => {
                        log::warn!("Camera initialization failed - neither libcamera-still nor raspistill found: {}", e);
                        self.is_available = false;
                        Ok(()) // Don't fail completely, just disable camera
                    }
                }
            }
        }
    }

    /// Take a photo and return it as an RgbImage
    pub async fn take_photo(&mut self) -> Result<RgbImage> {
        if !self.is_available {
            // Create a dummy image for development/testing when camera is not available
            log::warn!("Camera not available - creating test pattern");
            let img = ImageBuffer::from_fn(self.width, self.height, |x, y| {
                let r = (x * 255 / self.width) as u8;
                let g = (y * 255 / self.height) as u8;
                let b = ((x + y) * 255 / (self.width + self.height)) as u8;
                image::Rgb([r, g, b])
            });
            return Ok(img);
        }

        log::info!("Taking photo with Pi Camera...");
        
        // Remove any existing temp file
        if Path::new(&self.temp_image_path).exists() {
            let _ = fs::remove_file(&self.temp_image_path).await;
        }

        // Give the camera a moment to adjust exposure
        sleep(Duration::from_millis(500)).await;
        
        // Try libcamera-still first (modern approach)
        let capture_result = Command::new("libcamera-still")
            .args(&[
                "-o", &self.temp_image_path,
                "--width", &self.width.to_string(),
                "--height", &self.height.to_string(),
                "--quality", &self.quality.to_string(),
                "--immediate",  // Take photo immediately without preview
                "--nopreview",  // Disable preview window
                "--timeout", "1000"  // 1 second timeout
            ])
            .output();

        let success = match capture_result {
            Ok(output) => {
                if output.status.success() {
                    true
                } else {
                    log::warn!("libcamera-still failed, trying raspistill fallback");
                    // Try legacy raspistill as fallback
                    let legacy_result = Command::new("raspistill")
                        .args(&[
                            "-o", &self.temp_image_path,
                            "-w", &self.width.to_string(),
                            "-h", &self.height.to_string(),
                            "-q", &self.quality.to_string(),
                            "-t", "1000",  // 1 second timeout
                            "-n"   // No preview
                        ])
                        .output();
                    
                    match legacy_result {
                        Ok(output) => output.status.success(),
                        Err(_) => false
                    }
                }
            }
            Err(_) => false
        };

        if !success {
            return Err(anyhow!("Failed to capture image with camera"));
        }

        // Load and decode the captured image
        match image::open(&self.temp_image_path) {
            Ok(img) => {
                let rgb_img = img.to_rgb8();
                log::info!("Photo captured successfully: {}x{}", rgb_img.width(), rgb_img.height());
                
                // Clean up temp file
                let _ = fs::remove_file(&self.temp_image_path).await;
                
                Ok(rgb_img)
            }
            Err(e) => {
                Err(anyhow!("Failed to load captured image: {}", e))
            }
        }
    }

    /// Save a photo to disk
    pub async fn take_and_save_photo(&mut self, path: &Path) -> Result<RgbImage> {
        let img = self.take_photo().await?;
        
        // Save the image
        img.save(path).map_err(|e| {
            anyhow!("Failed to save image to {:?}: {}", path, e)
        })?;
        
        log::info!("Photo saved to {:?}", path);
        Ok(img)
    }

    /// Set camera resolution
    pub fn set_resolution(&mut self, width: u32, height: u32) -> Result<()> {
        self.width = width;
        self.height = height;
        
        // Reinitialize camera with new settings
        self.initialize()
    }

    /// Set JPEG quality (0-100)
    pub fn set_quality(&mut self, quality: u8) {
        self.quality = quality.min(100);
    }

    /// Check if camera is available and working
    pub fn is_available(&self) -> bool {
        self.is_available
    }

    /// Get current camera settings
    pub fn get_settings(&self) -> (u32, u32, u8) {
        (self.width, self.height, self.quality)
    }
}

impl Drop for CameraController {
    fn drop(&mut self) {
        // Clean up any remaining temp files
        if Path::new(&self.temp_image_path).exists() {
            let _ = std::fs::remove_file(&self.temp_image_path);
        }
        log::info!("Camera controller dropped");
    }
}
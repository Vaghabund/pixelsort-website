use anyhow::{anyhow, Result};
use image::{RgbImage, ImageBuffer};
use std::path::Path;
use tokio::time::{Duration, sleep};

#[cfg(feature = "camera")]
use rascam::{SimpleCamera, CameraSettings};

/// Camera controller for Raspberry Pi Camera v1.5
pub struct CameraController {
    #[cfg(feature = "camera")]
    camera: Option<SimpleCamera>,
    
    /// Camera settings
    width: u32,
    height: u32,
    quality: u8,
}

impl CameraController {
    /// Create a new camera controller
    pub fn new() -> Result<Self> {
        let mut controller = CameraController {
            #[cfg(feature = "camera")]
            camera: None,
            width: 1024,  // Default resolution
            height: 768,
            quality: 85,  // JPEG quality (0-100)
        };

        controller.initialize()?;
        Ok(controller)
    }

    /// Initialize the camera
    pub fn initialize(&mut self) -> Result<()> {
        #[cfg(feature = "camera")]
        {
            let settings = CameraSettings {
                width: self.width,
                height: self.height,
                format: rascam::Format::JPEG,
                quality: self.quality,
                ..Default::default()
            };

            match SimpleCamera::new(settings) {
                Ok(camera) => {
                    self.camera = Some(camera);
                    log::info!("Raspberry Pi Camera v1.5 initialized successfully");
                    Ok(())
                }
                Err(e) => {
                    log::error!("Failed to initialize camera: {}", e);
                    Err(anyhow!("Camera initialization failed: {}", e))
                }
            }
        }

        #[cfg(not(feature = "camera"))]
        {
            log::warn!("Camera feature not enabled - camera functionality disabled");
            Ok(())
        }
    }

    /// Take a photo and return it as an RgbImage
    pub async fn take_photo(&mut self) -> Result<RgbImage> {
        #[cfg(feature = "camera")]
        {
            if let Some(ref mut camera) = self.camera {
                log::info!("Taking photo with Pi Camera...");
                
                // Give the camera a moment to adjust exposure
                sleep(Duration::from_millis(500)).await;
                
                // Capture the image
                let jpeg_data = camera.capture().map_err(|e| {
                    anyhow!("Failed to capture image: {}", e)
                })?;
                
                log::info!("Captured {} bytes of image data", jpeg_data.len());
                
                // Decode the JPEG data into an image
                let img = image::load_from_memory(&jpeg_data)
                    .map_err(|e| anyhow!("Failed to decode image: {}", e))?;
                
                // Convert to RGB format
                let rgb_img = img.to_rgb8();
                log::info!("Photo captured successfully: {}x{}", rgb_img.width(), rgb_img.height());
                
                Ok(rgb_img)
            } else {
                Err(anyhow!("Camera not initialized"))
            }
        }

        #[cfg(not(feature = "camera"))]
        {
            // Create a dummy image for development/testing
            log::warn!("Camera not available - creating test pattern");
            let img = ImageBuffer::from_fn(self.width, self.height, |x, y| {
                let r = (x * 255 / self.width) as u8;
                let g = (y * 255 / self.height) as u8;
                let b = ((x + y) * 255 / (self.width + self.height)) as u8;
                image::Rgb([r, g, b])
            });
            Ok(img)
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
        #[cfg(feature = "camera")]
        {
            self.camera.is_some()
        }

        #[cfg(not(feature = "camera"))]
        {
            false
        }
    }

    /// Get current camera settings
    pub fn get_settings(&self) -> (u32, u32, u8) {
        (self.width, self.height, self.quality)
    }
}

impl Drop for CameraController {
    fn drop(&mut self) {
        #[cfg(feature = "camera")]
        {
            if let Some(_) = self.camera.take() {
                log::info!("Camera controller dropped");
            }
        }
    }
}
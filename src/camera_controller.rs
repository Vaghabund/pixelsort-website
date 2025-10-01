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
    /// Preview image path for live feed
    preview_image_path: String,
    /// Whether rpicam-still is available
    is_available: bool,
    /// Preview process handle
    preview_process: Option<std::process::Child>,
}

impl CameraController {
    /// Create a new camera controller
    pub fn new() -> Result<Self> {
        let mut controller = CameraController {
            width: 800,  // Good size for preview
            height: 600,
            quality: 85,  // JPEG quality (0-100)
            temp_image_path: "/tmp/pixelsort_camera_capture.jpg".to_string(),
            preview_image_path: "/tmp/pixelsort_camera_preview.jpg".to_string(),
            is_available: false,
            preview_process: None,
        };

        controller.initialize()?;
        Ok(controller)
    }

    /// Initialize the camera by checking if rpicam-still is available
    pub fn initialize(&mut self) -> Result<()> {
        log::info!("Initializing camera controller...");
        
        // Check if rpicam-still command is available
        match Command::new("rpicam-still").arg("--help").output() {
            Ok(output) => {
                self.is_available = true;
                log::info!("Raspberry Pi Camera initialized successfully (using rpicam-still)");
                log::debug!("rpicam-still help output: {}", String::from_utf8_lossy(&output.stdout));
                Ok(())
            }
            Err(e) => {
                log::warn!("rpicam-still not found: {}", e);
                // Try legacy raspistill as fallback
                match Command::new("raspistill").arg("-?").output() {
                    Ok(_) => {
                        self.is_available = true;
                        log::info!("Raspberry Pi Camera initialized successfully (using legacy raspistill)");
                        Ok(())
                    }
                    Err(e) => {
                        log::error!("Camera initialization failed - neither rpicam-still nor raspistill found: {}", e);
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
            log::debug!("Removed existing temp file");
        }

        // Give the camera a moment to adjust exposure
        sleep(Duration::from_millis(500)).await;
        
        // Try rpicam-still first (modern approach)
        let args = [
            "-o", &self.temp_image_path,
            "--width", &self.width.to_string(),
            "--height", &self.height.to_string(),
            "--quality", &self.quality.to_string(),
            "--immediate",  // Take photo immediately without preview
            "--nopreview",  // Disable preview window
            "--timeout", "1000"  // 1 second timeout
        ];
        
        log::info!("Capture command: rpicam-still {}", args.join(" "));
        
        let capture_result = Command::new("rpicam-still")
            .args(&args)
            .output();

        let success = match capture_result {
            Ok(output) => {
                if output.status.success() {
                    log::info!("rpicam-still capture successful");
                    true
                } else {
                    log::warn!("rpicam-still failed with status: {}", output.status);
                    log::warn!("stderr: {}", String::from_utf8_lossy(&output.stderr));
                    log::warn!("Trying raspistill fallback...");
                    
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
                        Ok(output) => {
                            if output.status.success() {
                                log::info!("raspistill fallback successful");
                                true
                            } else {
                                log::error!("raspistill also failed: {}", output.status);
                                false
                            }
                        }
                        Err(e) => {
                            log::error!("raspistill command failed: {}", e);
                            false
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("rpicam-still command failed: {}", e);
                false
            }
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

    /// Start live preview (continuous capture for preview)
    pub fn start_preview(&mut self) -> Result<()> {
        log::info!("Starting camera preview...");
        
        if !self.is_available {
            log::error!("Cannot start preview: Camera not available");
            return Err(anyhow!("Camera not available"));
        }

        // Stop any existing preview
        self.stop_preview();

        // Start rpicam in continuous preview mode
        let mut cmd = Command::new("rpicam-still");
        let args = [
            "-o", &self.preview_image_path,
            "--width", "800",  // Use smaller resolution for preview
            "--height", "600",
            "--quality", "70",  // Lower quality for faster preview
            "--timeout", "0",   // Continuous mode
            "--nopreview",      // No system preview window
            "--signal",         // Enable signal capture for updates
            "--loop"            // Continuous capture
        ];
        
        log::info!("Preview command: rpicam-still {}", args.join(" "));
        cmd.args(&args);

        match cmd.spawn() {
            Ok(child) => {
                self.preview_process = Some(child);
                log::info!("Camera preview started successfully");
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to start camera preview: {}", e);
                Err(anyhow!("Failed to start preview: {}", e))
            }
        }
    }

    /// Stop live preview
    pub fn stop_preview(&mut self) {
        if let Some(mut process) = self.preview_process.take() {
            let _ = process.kill();
            let _ = process.wait();
            log::info!("Camera preview stopped");
        }
    }

    /// Get the latest preview image (non-blocking)
    pub fn get_preview_image(&self) -> Result<RgbImage> {
        if !self.is_available {
            log::debug!("Camera not available, returning test pattern");
            // Return test pattern if camera not available
            let img = ImageBuffer::from_fn(800, 600, |x, y| {
                let time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f32();
                
                let r = ((x as f32 / 800.0 * 255.0) + (time * 50.0).sin() * 50.0) as u8;
                let g = ((y as f32 / 600.0 * 255.0) + (time * 30.0).cos() * 50.0) as u8;
                let b = (((x + y) as f32 / 1400.0 * 255.0) + (time * 70.0).sin() * 50.0) as u8;
                image::Rgb([r.saturating_add(100), g.saturating_add(100), b.saturating_add(100)])
            });
            return Ok(img);
        }

        // Check if preview file exists and get its metadata
        if !std::path::Path::new(&self.preview_image_path).exists() {
            log::warn!("Preview image file doesn't exist: {}", self.preview_image_path);
            return Err(anyhow!("Preview image file not found"));
        }

        // Try to load the preview image
        match image::open(&self.preview_image_path) {
            Ok(img) => {
                let rgb_img = img.to_rgb8();
                log::debug!("Successfully loaded preview image: {}x{}", rgb_img.width(), rgb_img.height());
                Ok(rgb_img)
            }
            Err(_) => {
                // If preview image doesn't exist yet, create a loading placeholder
                let img = ImageBuffer::from_fn(self.width, self.height, |x, y| {
                    if (x + y) % 50 < 25 {
                        image::Rgb([50, 50, 50])
                    } else {
                        image::Rgb([100, 100, 100])
                    }
                });
                Ok(img)
            }
        }
    }

    /// Take a quick snapshot (for pixel sorting) 
    pub fn capture_snapshot(&self) -> Result<RgbImage> {
        let temp_path = "/tmp/pixelsort_snapshot.jpg";
        
        // Remove any existing temp file
        if Path::new(temp_path).exists() {
            let _ = std::fs::remove_file(temp_path);
        }

        // Take a high-quality snapshot
        let result = Command::new("rpicam-still")
            .args(&[
                "-o", temp_path,
                "--width", &self.width.to_string(),
                "--height", &self.height.to_string(),
                "--quality", &self.quality.to_string(),
                "--immediate",
                "--nopreview",
                "--timeout", "100"  // Very quick capture
            ])
            .output();

        let success = match result {
            Ok(output) => output.status.success(),
            Err(_) => false
        };

        if !success {
            return Err(anyhow!("Failed to capture snapshot"));
        }

        // Load the image
        match image::open(temp_path) {
            Ok(img) => {
                let rgb_img = img.to_rgb8();
                // Clean up
                let _ = std::fs::remove_file(temp_path);
                Ok(rgb_img)
            }
            Err(e) => {
                Err(anyhow!("Failed to load snapshot: {}", e))
            }
        }
    }

    /// Get current camera settings
    pub fn get_settings(&self) -> (u32, u32, u8) {
        (self.width, self.height, self.quality)
    }
}

impl Drop for CameraController {
    fn drop(&mut self) {
        // Stop preview process
        self.stop_preview();
        
        // Clean up any remaining temp files
        if Path::new(&self.temp_image_path).exists() {
            let _ = std::fs::remove_file(&self.temp_image_path);
        }
        if Path::new(&self.preview_image_path).exists() {
            let _ = std::fs::remove_file(&self.preview_image_path);
        }
        log::info!("Camera controller dropped");
    }
}
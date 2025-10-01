use anyhow::{anyhow, Result};
use image::{RgbImage, ImageBuffer};
use std::path::Path;
use std::process::{Command, Child};
use std::time::{Duration, Instant};

/// Camera controller for Raspberry Pi Camera v1.5 using libcamera
/// Uses a hybrid approach: background preview stream + on-demand still capture
pub struct CameraController {
    /// Camera settings for still capture
    capture_width: u32,
    capture_height: u32,
    quality: u8,
    /// Preview settings (lower resolution for speed)
    preview_width: u32,
    preview_height: u32,
    /// Temporary file paths
    temp_capture_path: String,
    temp_preview_path: String,
    /// Whether rpicam commands are available
    is_available: bool,
    /// Background preview process (rpicam-vid or frame capture loop)
    preview_process: Option<Child>,
    /// Timing control for preview updates
    last_preview_update: Instant,
    preview_interval: Duration,
}

impl CameraController {
    /// Create a new camera controller
    pub fn new() -> Result<Self> {
        let mut controller = CameraController {
            // High resolution for final captures
            capture_width: 1024,
            capture_height: 768,
            quality: 90,  // High quality for pixel sorting
            // Lower resolution for smooth preview
            preview_width: 640,
            preview_height: 480,
            temp_capture_path: "/tmp/pixelsort_capture.jpg".to_string(),
            temp_preview_path: "/tmp/pixelsort_preview.jpg".to_string(),
            is_available: false,
            preview_process: None,
            last_preview_update: Instant::now(),
            preview_interval: Duration::from_millis(100), // 10 FPS preview
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

    /// Set camera resolution
    pub fn set_resolution(&mut self, width: u32, height: u32) -> Result<()> {
        self.capture_width = width;
        self.capture_height = height;
        
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

    /// Start live preview using optimized approach
    pub fn start_preview(&mut self) -> Result<()> {
        log::info!("Starting camera preview system...");
        
        if !self.is_available {
            log::error!("Cannot start preview: Camera not available");
            return Err(anyhow!("Camera not available"));
        }

        // Clean up any existing process
        self.stop_preview();

        // We'll use on-demand preview capture with timing control
        // This avoids the X11 preview window crashes we saw in testing
        log::info!("Camera preview ready (using timed still captures with --nopreview)");
        Ok(())
    }

    /// Stop live preview
    pub fn stop_preview(&mut self) {
        if let Some(mut process) = self.preview_process.take() {
            let _ = process.kill();
            let _ = process.wait();
            log::info!("Camera preview stopped");
        }
    }

    /// Get the latest preview image with timing control (non-blocking)
    pub fn get_preview_image(&mut self) -> Result<RgbImage> {
        if !self.is_available {
            // Return animated test pattern if camera not available
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f32();
            
            let img = ImageBuffer::from_fn(self.preview_width, self.preview_height, |x, y| {
                let r = ((x as f32 / self.preview_width as f32 * 255.0) + (time * 50.0).sin() * 50.0) as u8;
                let g = ((y as f32 / self.preview_height as f32 * 255.0) + (time * 30.0).cos() * 50.0) as u8;
                let b = (((x + y) as f32 / (self.preview_width + self.preview_height) as f32 * 255.0) + (time * 70.0).sin() * 50.0) as u8;
                image::Rgb([r.saturating_add(100), g.saturating_add(100), b.saturating_add(100)])
            });
            return Ok(img);
        }

        // Throttle preview updates to avoid the lag we experienced
        let now = Instant::now();
        if now.duration_since(self.last_preview_update) < self.preview_interval {
            // Return last captured frame or placeholder
            return self.load_existing_preview_or_placeholder();
        }
        
        self.last_preview_update = now;

        // Capture fresh preview using the working --nopreview approach
        let result = Command::new("rpicam-still")
            .args(&[
                "-o", &self.temp_preview_path,
                "--width", &self.preview_width.to_string(),
                "--height", &self.preview_height.to_string(),
                "--quality", "50",  // Lower quality for speed
                "--timeout", "50",  // Quick capture based on our tests
                "--nopreview",      // No X11 window (avoids crashes)
                "--immediate"       // Take photo immediately
            ])
            .output();

        match result {
            Ok(output) => {
                if !output.status.success() {
                    log::warn!("Preview capture failed: {}", String::from_utf8_lossy(&output.stderr));
                    return self.load_existing_preview_or_placeholder();
                }
            }
            Err(e) => {
                log::error!("Preview command failed: {}", e);
                return self.load_existing_preview_or_placeholder();
            }
        }

        // Load the captured preview
        match image::open(&self.temp_preview_path) {
            Ok(img) => {
                let rgb_img = img.to_rgb8();
                log::debug!("Preview captured: {}x{}", rgb_img.width(), rgb_img.height());
                Ok(rgb_img)
            }
            Err(e) => {
                log::debug!("Failed to load preview image: {}", e);
                self.load_existing_preview_or_placeholder()
            }
        }
    }

    /// Load existing preview file or return placeholder
    fn load_existing_preview_or_placeholder(&self) -> Result<RgbImage> {
        // Try to load existing preview file
        if let Ok(img) = image::open(&self.temp_preview_path) {
            return Ok(img.to_rgb8());
        }

        // Return placeholder pattern
        let img = ImageBuffer::from_fn(self.preview_width, self.preview_height, |x, y| {
            if (x + y) % 40 < 20 {
                image::Rgb([60, 60, 60])
            } else {
                image::Rgb([80, 80, 80])
            }
        });
        Ok(img)
    }

    /// Take a high-quality snapshot for pixel sorting
    pub fn capture_snapshot(&self) -> Result<RgbImage> {
        if !self.is_available {
            return Err(anyhow!("Camera not available"));
        }

        // Remove any existing capture file
        if Path::new(&self.temp_capture_path).exists() {
            let _ = std::fs::remove_file(&self.temp_capture_path);
        }

        log::info!("Taking high-quality snapshot for pixel sorting...");

        // Take a high-quality snapshot using the working approach
        let result = Command::new("rpicam-still")
            .args(&[
                "-o", &self.temp_capture_path,
                "--width", &self.capture_width.to_string(),
                "--height", &self.capture_height.to_string(),
                "--quality", &self.quality.to_string(),
                "--immediate",
                "--nopreview",
                "--timeout", "1000"  // 1 second for high quality
            ])
            .output();

        match result {
            Ok(output) => {
                if !output.status.success() {
                    log::error!("Snapshot capture failed: {}", String::from_utf8_lossy(&output.stderr));
                    return Err(anyhow!("rpicam-still failed"));
                }
            }
            Err(e) => {
                log::error!("Failed to execute rpicam-still: {}", e);
                return Err(anyhow!("Command execution failed: {}", e));
            }
        }

        // Load and return the captured image
        match image::open(&self.temp_capture_path) {
            Ok(img) => {
                let rgb_img = img.to_rgb8();
                log::info!("Snapshot captured successfully: {}x{}", rgb_img.width(), rgb_img.height());
                // Clean up temp file
                let _ = std::fs::remove_file(&self.temp_capture_path);
                Ok(rgb_img)
            }
            Err(e) => {
                Err(anyhow!("Failed to load snapshot: {}", e))
            }
        }
    }

    /// Get current camera settings
    pub fn get_settings(&self) -> (u32, u32, u8) {
        (self.capture_width, self.capture_height, self.quality)
    }
}

impl Drop for CameraController {
    fn drop(&mut self) {
        // Stop preview process
        self.stop_preview();
        
        // Clean up any remaining temp files
        if Path::new(&self.temp_capture_path).exists() {
            let _ = std::fs::remove_file(&self.temp_capture_path);
        }
        if Path::new(&self.temp_preview_path).exists() {
            let _ = std::fs::remove_file(&self.temp_preview_path);
        }
        log::info!("Camera controller dropped");
    }
}
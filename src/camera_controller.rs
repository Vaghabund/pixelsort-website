#![allow(dead_code)]
use anyhow::{anyhow, Result};
use image::{RgbImage, ImageBuffer};
use std::path::Path;
use std::io::Read;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Camera controller for Raspberry Pi Camera v1.5 using libcamera
/// Uses streaming approach for live preview + on-demand still capture
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
    /// Streaming camera process for live preview
    stream_process: Option<std::process::Child>,
    /// Channel for receiving frames from streaming thread
    frame_receiver: Option<Receiver<RgbImage>>,
    /// Channel for sending frames to main thread
    frame_sender: Option<Sender<RgbImage>>,
    /// Streaming thread handle
    stream_thread: Option<thread::JoinHandle<()>>,
    /// Whether streaming is active
    streaming_active: bool,
}

impl CameraController {
    /// Create a new camera controller
    pub fn new() -> Result<Self> {
        let mut controller = CameraController {
            // Match screen resolution for consistent display
            capture_width: 1024,
            capture_height: 600,
            quality: 90,  // High quality for pixel sorting
            // Match screen resolution (1024x600) for full-screen preview
            preview_width: 1024,
            preview_height: 600,
            temp_capture_path: "/tmp/pixelsort_capture.jpg".to_string(),
            temp_preview_path: "/tmp/pixelsort_preview.jpg".to_string(),
            is_available: false,
            stream_process: None,
            frame_receiver: None,
            frame_sender: None,
            stream_thread: None,
            streaming_active: false,
        };

        controller.initialize()?;
        Ok(controller)
    }

    /// Initialize the camera by checking if rpicam-still is available
    pub fn initialize(&mut self) -> Result<()> {
        // Check if rpicam-still command is available
        match Command::new("rpicam-still").arg("--help").output() {
            Ok(_) => {
                self.is_available = true;
                log::info!("Raspberry Pi Camera initialized successfully (using rpicam-still)");
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

    /// Start continuous camera streaming for live preview
    pub fn start_streaming(&mut self) -> Result<()> {
        if !self.is_available || self.streaming_active {
            return Ok(());
        }

        // Create channel for frame communication
        let (sender, receiver) = mpsc::channel();
        self.frame_sender = Some(sender);
        self.frame_receiver = Some(receiver);

        // Start streaming process
        let mut process = Command::new("rpicam-vid")
            .args(&[
                "--output", "-",  // Output to stdout
                "--width", &self.preview_width.to_string(),
                "--height", &self.preview_height.to_string(),
                "--framerate", "30",  // 30 FPS streaming
                "--codec", "mjpeg",  // MJPEG for individual frames
                "--nopreview",
                "--timeout", "0",  // Stream indefinitely
                "--flush", "1",    // Flush each frame
            ])
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        self.stream_process = Some(process);

        // Start background thread to read frames
        let frame_sender = self.frame_sender.as_ref().unwrap().clone();
        let mut stdout = self.stream_process.as_mut().unwrap().stdout.take().unwrap();

        let stream_thread = thread::spawn(move || {
            let mut buffer = Vec::new();
            let mut jpeg_start = false;

            loop {
                let mut temp_buf = [0u8; 4096];
                match stdout.read(&mut temp_buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        buffer.extend_from_slice(&temp_buf[..n]);

                        // Look for JPEG markers
                        while let Some(start_pos) = buffer.windows(2).position(|w| w == [0xFF, 0xD8]) {
                            if let Some(end_pos) = buffer[start_pos + 2..].windows(2).position(|w| w == [0xFF, 0xD9]) {
                                let jpeg_end = start_pos + 2 + end_pos + 2;
                                if jpeg_end <= buffer.len() {
                                    let jpeg_data = &buffer[start_pos..jpeg_end];

                                    // Decode JPEG frame
                                    if let Ok(img) = image::load_from_memory_with_format(jpeg_data, image::ImageFormat::Jpeg) {
                                        let rgb_img = img.to_rgb8();
                                        // Send frame to main thread (non-blocking)
                                        let _ = frame_sender.send(rgb_img);
                                    }

                                    // Remove processed data
                                    buffer.drain(0..jpeg_end);
                                } else {
                                    break; // Incomplete frame
                                }
                            } else {
                                break; // No end marker found
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        self.stream_thread = Some(stream_thread);
        self.streaming_active = true;

        log::info!("Camera streaming started at {}x{} @ 30 FPS", self.preview_width, self.preview_height);
        Ok(())
    }

    /// Stop camera streaming
    pub fn stop_streaming(&mut self) {
        self.streaming_active = false;

        // Kill the streaming process
        if let Some(mut process) = self.stream_process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }

        // Join the streaming thread
        if let Some(thread) = self.stream_thread.take() {
            let _ = thread.join();
        }

        // Clear channels
        self.frame_sender = None;
        self.frame_receiver = None;

        log::info!("Camera streaming stopped");
    }

    /// Get fast live preview image from streaming camera
    pub fn get_fast_preview_image(&mut self) -> Result<RgbImage> {
        if !self.is_available {
            return self.get_test_pattern();
        }

        // Start streaming if not already active
        if !self.streaming_active {
            self.start_streaming()?;
            // Give streaming a moment to start
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // Try to get latest frame from stream
        if let Some(receiver) = &self.frame_receiver {
            // Drain old frames, keep only the latest
            let mut latest_frame = None;
            while let Ok(frame) = receiver.try_recv() {
                latest_frame = Some(frame);
            }

            if let Some(frame) = latest_frame {
                return Ok(frame);
            }
        }

        // Fallback to test pattern if no frames available
        self.get_test_pattern()
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

        // Throttle preview updates to avoid lag
        let now = Instant::now();
        if now.duration_since(self.last_preview_update) < self.preview_interval {
            // Return last captured frame or placeholder
            return self.load_existing_preview_or_placeholder();
        }
        
        self.last_preview_update = now;

        // Delete existing temp file to force fresh capture
        if std::path::Path::new(&self.temp_preview_path).exists() {
            let _ = std::fs::remove_file(&self.temp_preview_path);
        }

        // Capture fresh preview using the working --nopreview approach
        let result = Command::new("rpicam-still")
            .args(&[
                "-o", &self.temp_preview_path,
                "--width", &self.preview_width.to_string(),
                "--height", &self.preview_height.to_string(),
                "--quality", "50",  // Lower quality for speed
                "--timeout", "30",  // Faster timeout for higher FPS
                "--nopreview",      // No X11 window (avoids crashes)
                "--immediate",      // Take photo immediately
                "--flush"           // Flush any cached frames
            ])
            .output();

        match result {
            Ok(output) => {
                if !output.status.success() {
                    return self.load_existing_preview_or_placeholder();
                }
            }
            Err(_) => {
                return self.load_existing_preview_or_placeholder();
            }
        }

        // Load the captured preview
        match image::open(&self.temp_preview_path) {
            Ok(img) => {
                let rgb_img = img.to_rgb8();
                
                // Basic validation: check if image is completely solid color (likely corrupted)
                if self.is_likely_corrupted(&rgb_img) {
                    // Try one more capture
                    let _ = std::fs::remove_file(&self.temp_preview_path);
                    let retry_result = Command::new("rpicam-still")
                        .args(&[
                            "-o", &self.temp_preview_path,
                            "--width", &self.preview_width.to_string(),
                            "--height", &self.preview_height.to_string(),
                            "--quality", "50",
                            "--timeout", "200", // Longer timeout for retry
                            "--nopreview",
                            "--immediate"
                        ])
                        .output();
                        
                    if retry_result.is_ok() {
                        if let Ok(retry_img) = image::open(&self.temp_preview_path) {
                            let retry_rgb = retry_img.to_rgb8();
                            if !self.is_likely_corrupted(&retry_rgb) {
                                return Ok(retry_rgb);
                            }
                        }
                    }
                }
                
                Ok(rgb_img)
            }
            Err(_) => {
                self.load_existing_preview_or_placeholder()
            }
        }
    }

    /// Get animated test pattern when camera not available
    fn get_test_pattern(&self) -> Result<RgbImage> {
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
        Ok(img)
    }

    /// Check if image is likely corrupted (solid color, etc.)
    fn is_likely_corrupted(&self, img: &RgbImage) -> bool {
        if img.width() < 10 || img.height() < 10 {
            return true; // Too small
        }
        
        // Sample a few pixels to check for solid color (common corruption)
        let sample_pixels = [
            img.get_pixel(img.width() / 4, img.height() / 4),
            img.get_pixel(img.width() / 2, img.height() / 2),
            img.get_pixel(3 * img.width() / 4, 3 * img.height() / 4),
            img.get_pixel(10, 10),
            img.get_pixel(img.width() - 10, img.height() - 10),
        ];
        
        // If all sampled pixels are identical, likely corrupted
        let first_pixel = sample_pixels[0];
        sample_pixels.iter().all(|&pixel| pixel == first_pixel)
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
                    return Err(anyhow!("rpicam-still failed"));
                }
            }
            Err(e) => {
                return Err(anyhow!("Command execution failed: {}", e));
            }
        }

        // Load and return the captured image
        match image::open(&self.temp_capture_path) {
            Ok(img) => {
                let rgb_img = img.to_rgb8();
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
        // Stop streaming process
        self.stop_streaming();

        // Clean up any remaining temp files
        if Path::new(&self.temp_capture_path).exists() {
            let _ = std::fs::remove_file(&self.temp_capture_path);
        }
        if Path::new(&self.temp_preview_path).exists() {
            let _ = std::fs::remove_file(&self.temp_preview_path);
        }
    }
}
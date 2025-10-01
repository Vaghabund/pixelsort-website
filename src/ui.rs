use eframe::egui::{self, ColorImage, TextureHandle};
use image::RgbImage;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

use crate::config::Config;
use crate::gpio_controller::GpioController;
use crate::image_processor::ImageProcessor;
use crate::pixel_sorter::{PixelSorter, SortingAlgorithm, SortingParameters};
use crate::camera_controller::CameraController;

pub struct PixelSorterApp {
    pixel_sorter: Arc<PixelSorter>,
    image_processor: Arc<RwLock<ImageProcessor>>,
    gpio_controller: Option<Arc<RwLock<GpioController>>>,
    camera_controller: Option<Arc<RwLock<CameraController>>>,
    config: Config,
    
    // UI State
    current_algorithm: SortingAlgorithm,
    sorting_params: SortingParameters,
    
    // Image state
    original_image: Option<RgbImage>,
    processed_image: Option<RgbImage>,
    preview_image: Option<RgbImage>,
    image_texture: Option<TextureHandle>,
    preview_texture: Option<TextureHandle>,
    
    // UI flags
    is_processing: bool,
    status_message: String,
    show_file_dialog: bool,
    preview_mode: bool,  // Whether showing live preview or processed image
}

impl PixelSorterApp {
    pub fn new(
        pixel_sorter: Arc<PixelSorter>,
        image_processor: Arc<RwLock<ImageProcessor>>,
        gpio_controller: Option<Arc<RwLock<GpioController>>>,
        camera_controller: Option<Arc<RwLock<CameraController>>>,
        config: Config,
    ) -> Self {
        let status_msg = if camera_controller.is_some() {
            "Camera Preview Active - Press any button to capture and sort!"
        } else {
            "Ready - Load an image to begin"
        };

        let mut app = Self {
            pixel_sorter,
            image_processor,
            gpio_controller,
            camera_controller,
            config,
            current_algorithm: SortingAlgorithm::Horizontal,
            sorting_params: SortingParameters::default(),
            original_image: None,
            processed_image: None,
            preview_image: None,
            image_texture: None,
            preview_texture: None,
            is_processing: false,
            status_message: status_msg.to_string(),
            show_file_dialog: false,
            preview_mode: true,  // Start in preview mode
        };

        // Note: Camera preview will be started in the first update loop

        app
    }

    fn load_image(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg", "bmp", "gif", "tiff"])
            .pick_file()
        {
            match image::open(&path) {
                Ok(img) => {
                    let rgb_img = img.to_rgb8();
                    self.original_image = Some(rgb_img);
                    self.status_message = format!("Loaded: {}", path.file_name().unwrap_or_default().to_string_lossy());
                    self.process_image(ctx);
                }
                Err(e) => {
                    self.status_message = format!("Failed to load image: {}", e);
                }
            }
        }
    }

    fn save_image(&mut self) {
        if let Some(ref processed_img) = self.processed_image {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG", &["png"])
                .add_filter("JPEG", &["jpg"])
                .set_file_name("sorted_image.png")
                .save_file()
            {
                match processed_img.save(&path) {
                    Ok(_) => {
                        self.status_message = format!("Saved: {}", path.file_name().unwrap_or_default().to_string_lossy());
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to save image: {}", e);
                    }
                }
            }
        } else {
            self.status_message = "No processed image to save".to_string();
        }
    }

    fn capture_and_sort(&mut self, ctx: &egui::Context) {
        if let Some(ref camera) = self.camera_controller {
            self.is_processing = true;
            self.preview_mode = false;  // Switch to processed image view
            self.status_message = "Capturing and sorting...".to_string();
            
            // Capture snapshot from camera
            let capture_result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let camera_lock = camera.read().await;
                    camera_lock.capture_snapshot()
                })
            });
            
            match capture_result {
                Ok(rgb_img) => {
                    self.original_image = Some(rgb_img);
                    self.status_message = format!("Captured! Applying {} sorting...", self.current_algorithm);
                    self.process_image(ctx);
                }
                Err(e) => {
                    self.status_message = format!("Capture failed: {}", e);
                    self.is_processing = false;
                    self.preview_mode = true;  // Back to preview mode
                }
            }
        } else {
            self.status_message = "Camera not available".to_string();
        }
    }

    fn take_photo_blocking(&self) -> Result<image::RgbImage, anyhow::Error> {
        use std::process::Command;
        use anyhow::anyhow;
        
        let temp_path = "/tmp/pixelsort_capture.jpg";
        
        // Remove any existing temp file
        if std::path::Path::new(temp_path).exists() {
            let _ = std::fs::remove_file(temp_path);
        }

        // Try libcamera-still first
        let result = Command::new("libcamera-still")
            .args(&[
                "-o", temp_path,
                "--width", "1024",
                "--height", "768", 
                "--quality", "85",
                "--immediate",
                "--nopreview",
                "--timeout", "1000"
            ])
            .output();

        let success = match result {
            Ok(output) => {
                if output.status.success() {
                    true
                } else {
                    // Try raspistill fallback
                    let legacy_result = Command::new("raspistill")
                        .args(&["-o", temp_path, "-w", "1024", "-h", "768", "-q", "85", "-t", "1000", "-n"])
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
            return Err(anyhow!("Failed to capture photo with camera"));
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
                Err(anyhow!("Failed to load captured image: {}", e))
            }
        }
    }

    fn process_image(&mut self, ctx: &egui::Context) {
        if let Some(ref original) = self.original_image {
            if self.is_processing {
                return;
            }

            self.is_processing = true;
            self.status_message = "Processing...".to_string();

            match self.pixel_sorter.sort_pixels(original, self.current_algorithm, &self.sorting_params) {
                Ok(processed) => {
                    self.processed_image = Some(processed.clone());
                    self.update_image_texture(ctx, &processed);
                    self.status_message = "Processing complete".to_string();
                }
                Err(e) => {
                    self.status_message = format!("Processing failed: {}", e);
                }
            }

            self.is_processing = false;
        }
    }

    fn update_image_texture(&mut self, ctx: &egui::Context, image: &RgbImage) {
        let (width, height) = image.dimensions();
        let pixels: Vec<egui::Color32> = image
            .pixels()
            .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
            .collect();

        let color_image = ColorImage {
            size: [width as usize, height as usize],
            pixels,
        };

        self.processed_texture = Some(ctx.load_texture("processed_image", color_image, egui::TextureOptions::default()));
    }

    fn update_preview(&mut self, ctx: &egui::Context) {
        if let Some(ref camera) = self.camera_controller {
            // Get latest preview image
            let preview_result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let camera_lock = camera.read().await;
                    camera_lock.get_preview_image()
                })
            });

            match preview_result {
                Ok(rgb_img) => {
                    self.preview_image = Some(rgb_img.clone());
                    
                    // Update preview texture
                    let (width, height) = rgb_img.dimensions();
                    let pixels: Vec<egui::Color32> = rgb_img
                        .pixels()
                        .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
                        .collect();

                    let color_image = ColorImage {
                        size: [width as usize, height as usize],
                        pixels,
                    };

                    self.preview_texture = Some(ctx.load_texture("preview_image", color_image, egui::TextureOptions::default()));
                }
                Err(_) => {
                    // Silently fail - preview updates can be intermittent
                }
            }
        }
    }

    fn handle_gpio_input(&mut self, ctx: &egui::Context) {
        if let Some(ref gpio_controller) = self.gpio_controller {
            // In a real implementation, you'd poll for button presses here
            // For now, we'll handle this in the GPIO controller itself
        }
    }

    pub fn on_button_press(&mut self, button_id: u8, ctx: &egui::Context) {
        match button_id {
            1 => self.load_image(ctx), // Load image
            2 => self.capture_and_sort(ctx), // Capture and sort (camera)
            3 => {
                // Next algorithm
                self.current_algorithm = self.current_algorithm.next();
                if !self.preview_mode && self.original_image.is_some() {
                    self.process_image(ctx);
                }
            }
            4 => {
                // Threshold up
                self.sorting_params.threshold = (self.sorting_params.threshold + 10.0).min(255.0);
                if !self.preview_mode && self.original_image.is_some() {
                    self.process_image(ctx);
                }
            }
            5 => {
                // Threshold down
                self.sorting_params.threshold = (self.sorting_params.threshold - 10.0).max(0.0);
                if !self.preview_mode && self.original_image.is_some() {
                    self.process_image(ctx);
                }
            }
            6 => self.save_image(), // Save image
            _ => {}
        }
    }
}

impl eframe::App for PixelSorterApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Handle GPIO input
        self.handle_gpio_input(ctx);

        // Update live preview if in preview mode
        if self.preview_mode {
            self.update_preview(ctx);
        }

        // Make sure we use the full screen area
        egui::CentralPanel::default()
            .frame(egui::Frame::none()) // Remove any padding/margins
            .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left panel - Image display
                ui.vertical(|ui| {
                    if self.preview_mode {
                        ui.heading("Live Camera Preview");
                    } else {
                        ui.heading("Pixel Sorted Result");
                    }
                    
                    // Display appropriate image based on mode
                    if self.is_processing {
                        let placeholder_size = egui::vec2(400.0, 300.0);
                        ui.allocate_ui_with_layout(
                            placeholder_size,
                            egui::Layout::centered_and_justified(egui::Direction::TopDown),
                            |ui| {
                                ui.spinner();
                                ui.label("Processing...");
                            },
                        );
                    } else if self.preview_mode {
                        // Show live camera preview
                        if let Some(ref texture) = self.preview_texture {
                            let available_size = ui.available_size();
                            let image_size = texture.size_vec2();
                            
                            // Calculate display size maintaining aspect ratio
                            let scale = (available_size.x / image_size.x).min(available_size.y / image_size.y).min(1.0);
                            let display_size = image_size * scale;
                            
                            ui.add(
                                egui::Image::from_texture(texture)
                                    .fit_to_exact_size(display_size)
                                    .rounding(egui::Rounding::same(8.0))
                            );
                        } else {
                            let placeholder_size = egui::vec2(400.0, 300.0);
                            ui.allocate_ui_with_layout(
                                placeholder_size,
                                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                |ui| {
                                    ui.label("Starting camera...");
                                    ui.spinner();
                                },
                            );
                        }
                    } else if let Some(ref texture) = self.processed_texture {
                        // Show processed image
                        let available_size = ui.available_size();
                        let image_size = texture.size_vec2();
                        
                        // Calculate display size maintaining aspect ratio
                        let scale = (available_size.x / image_size.x).min(available_size.y / image_size.y).min(1.0);
                        let display_size = image_size * scale;
                        
                        ui.add(
                            egui::Image::from_texture(texture)
                                .fit_to_exact_size(display_size)
                                .rounding(egui::Rounding::same(8.0))
                        );
                    } else {
                        // Fallback
                        let placeholder_size = egui::vec2(400.0, 300.0);
                        ui.allocate_ui_with_layout(
                            placeholder_size,
                            egui::Layout::centered_and_justified(egui::Direction::TopDown),
                            |ui| {
                                ui.label("No image available");
                                ui.label("Camera should start automatically");
                            },
                        );
                    }
                });

                ui.separator();

                // Right panel - Controls
                ui.vertical(|ui| {
                    ui.set_width(250.0);
                    ui.heading("Controls");

                    ui.add_space(10.0);

                    // Main action buttons
                    if self.preview_mode {
                        let capture_button = egui::Button::new("� Capture & Sort").min_size([200.0, 50.0].into());
                        if ui.add_enabled(!self.is_processing, capture_button).clicked() {
                            self.capture_and_sort(ctx);
                        }
                    } else {
                        if ui.add_sized([200.0, 50.0], egui::Button::new("📷 Back to Preview")).clicked() {
                            self.preview_mode = true;
                            self.status_message = "Live preview active - Press button to capture!".to_string();
                        }
                    }

                    if ui.add_sized([200.0, 50.0], egui::Button::new("📁 Load Image")).clicked() {
                        self.load_image(ctx);
                    }

                    if ui.add_sized([200.0, 50.0], egui::Button::new("💾 Save Result")).clicked() {
                        self.save_image();
                    }

                    ui.add_space(10.0);

                    if ui.add_sized([200.0, 50.0], egui::Button::new("�️ Force Fullscreen")).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                    }

                    if ui.add_sized([200.0, 50.0], egui::Button::new("�🚪 Exit")).clicked() {
                        std::process::exit(0);
                    }

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Algorithm selection
                    ui.label("Sorting Algorithm:");
                    ui.add_space(5.0);

                    for &algorithm in SortingAlgorithm::all() {
                        if ui.add_sized(
                            [200.0, 40.0],
                            egui::RadioButton::new(
                                std::mem::discriminant(&self.current_algorithm) == std::mem::discriminant(&algorithm),
                                algorithm.name(),
                            ),
                        ).clicked() {
                            self.current_algorithm = algorithm;
                            self.process_image(ctx);
                        }
                    }

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Parameter controls
                    ui.label("Parameters:");
                    ui.add_space(5.0);

                    ui.label(format!("Threshold: {:.1}", self.sorting_params.threshold));
                    let threshold_changed = ui.add(
                        egui::Slider::new(&mut self.sorting_params.threshold, 0.0..=255.0)
                            .step_by(1.0)
                    ).changed();

                    ui.add_space(10.0);

                    ui.label(format!("Interval: {}", self.sorting_params.interval));
                    let interval_changed = ui.add(
                        egui::Slider::new(&mut self.sorting_params.interval, 1..=50)
                    ).changed();

                    // Auto-process when parameters change
                    if (threshold_changed || interval_changed) && !self.is_processing {
                        self.process_image(ctx);
                    }

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Status display
                    ui.label("Status:");
                    ui.label(&self.status_message);
                    
                    // Debug info for window size
                    let screen_rect = ctx.screen_rect();
                    ui.label(format!("Screen: {:.0}×{:.0}", screen_rect.width(), screen_rect.height()));

                    if self.is_processing {
                        ui.add_space(10.0);
                        ui.spinner();
                    }

                    ui.add_space(20.0);

                    // GPIO button indicators
                    if self.gpio_controller.is_some() {
                        ui.separator();
                        ui.add_space(10.0);
                        ui.label("GPIO Buttons:");
                        ui.label("1: Load Image");
                        if self.camera_controller.is_some() {
                            ui.label("2: Capture & Sort �");
                            ui.label("3: Next Algorithm");
                            ui.label("4: Threshold ↑");
                            ui.label("5: Threshold ↓");
                            ui.label("6: Save Image");
                        } else {
                            ui.label("2: Next Algorithm");
                            ui.label("3: Threshold ↑");
                            ui.label("4: Threshold ↓");
                            ui.label("5: Save Image");
                        }
                        ui.label("ESC or Exit Button: Quit");
                    } else {
                        ui.separator();
                        ui.add_space(10.0);
                        ui.label("Keyboard Shortcuts:");
                        if self.camera_controller.is_some() {
                            ui.label("1-6: Button functions");
                        } else {
                            ui.label("1-5: Button functions");
                        }
                        ui.label("ESC or Exit Button: Quit");
                    }
                });
            });
        });

        // Handle keyboard input for development
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Key { key, pressed: true, .. } = event {
                    match key {
                        egui::Key::Num1 => self.on_button_press(1, ctx),
                        egui::Key::Num2 => self.on_button_press(2, ctx),
                        egui::Key::Num3 => self.on_button_press(3, ctx),
                        egui::Key::Num4 => self.on_button_press(4, ctx),
                        egui::Key::Num5 => self.on_button_press(5, ctx),
                        egui::Key::Num6 => self.on_button_press(6, ctx),
                        egui::Key::Escape => std::process::exit(0),
                        _ => {}
                    }
                }
            }
        });

        // Request repaint for smooth updates
        ctx.request_repaint();
    }
}
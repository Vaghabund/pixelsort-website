use std::sync::Arc;
use std::path::PathBuf;
use std::time::Instant;
use eframe::egui;
use image;
use rfd;
use tokio::sync::RwLock;
use chrono::{DateTime, Local};

use crate::pixel_sorter::{PixelSorter, SortingAlgorithm, SortingParameters};
use crate::camera_controller::CameraController;
use crate::gpio_controller::GpioController;
use crate::image_processor::ImageProcessor;
use crate::config::Config;

pub struct PixelSorterApp {
    pub original_image: Option<image::RgbImage>,
    pub processed_image: Option<image::RgbImage>,
    pub camera_texture: Option<egui::TextureHandle>,
    pub processed_texture: Option<egui::TextureHandle>,
    pub pixel_sorter: Arc<PixelSorter>,
    pub current_algorithm: SortingAlgorithm,
    pub sorting_params: SortingParameters,
    pub is_processing: bool,
    pub status_message: String,
    pub camera_controller: Option<Arc<RwLock<CameraController>>>,
    #[allow(dead_code)]
    pub gpio_controller: Option<Arc<RwLock<GpioController>>>,
    #[allow(dead_code)]
    pub image_processor: Arc<RwLock<ImageProcessor>>,
    #[allow(dead_code)]
    pub config: Config,
    pub preview_mode: bool,
    pub iteration_counter: u32,
    pub current_session_folder: Option<String>,
    pub last_camera_update: Option<Instant>,
    pub camera_image_data: Option<image::RgbImage>,
}

impl PixelSorterApp {
    pub fn new(
        pixel_sorter: Arc<PixelSorter>,
        image_processor: Arc<RwLock<ImageProcessor>>,
        gpio_controller: Option<Arc<RwLock<GpioController>>>,
        camera_controller: Option<Arc<RwLock<CameraController>>>,
        config: Config,
    ) -> Self {
        Self {
            original_image: None,
            processed_image: None,
            camera_texture: None,
            processed_texture: None,
            pixel_sorter,
            current_algorithm: SortingAlgorithm::Horizontal,
            sorting_params: SortingParameters::default(),
            is_processing: false,
            status_message: if camera_controller.is_some() {
                "Live preview active - Press button to capture!".to_string()
            } else {
                "Ready - Load an image to begin".to_string()
            },
            camera_controller,
            gpio_controller,
            image_processor,
            config,
            preview_mode: true,
            iteration_counter: 0,
            current_session_folder: None,
            last_camera_update: None,
            camera_image_data: None,
        }
    }

    // Camera preview is now handled directly in the main update loop with throttling
}

impl eframe::App for PixelSorterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Camera preview is now handled directly in the update loop below

        // Live camera updates with all stability fixes applied - only when in preview mode and not processing
        if self.preview_mode && self.camera_controller.is_some() && !self.is_processing {
            let now = Instant::now();
            let should_update = match self.last_camera_update {
                None => true,
                Some(last) => now.duration_since(last) >= std::time::Duration::from_millis(33), // 30 FPS - standard smooth video
            };

            if should_update {
                if let Some(ref camera) = self.camera_controller {
                    let preview_result = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            let mut camera_lock = camera.write().await;
                            camera_lock.get_fast_preview_image() // Use fast method for live preview
                        })
                    });

                    match preview_result {
                        Ok(preview_image) => {
                            self.camera_image_data = Some(preview_image.clone());
                            self.create_camera_texture(ctx, preview_image);
                            self.last_camera_update = Some(now);
                        }
                        Err(_) => {
                            // Silently ignore preview errors for better performance
                        }
                    }
                }
            }
        }

        // Remove the panel-based layout - we'll use overlay approach instead

        // Full-screen image display
        egui::CentralPanel::default().show(ctx, |ui| {
            let screen_rect = ui.max_rect();
            
            // Fill entire window with image
            if self.preview_mode {
                // Show camera preview or prompt
                if let Some(ref _camera) = self.camera_controller {
                    if let Some(texture) = &self.camera_texture {
                        // Fill entire window
                        ui.allocate_ui_at_rect(screen_rect, |ui| {
                            ui.add_sized(screen_rect.size(), egui::Image::new(texture));
                        });
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Initializing camera...");
                        });
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No camera available - Load an image to begin");
                    });
                }
            } else {
                // Show processed image full screen
                if let Some(texture) = &self.processed_texture {
                    ui.allocate_ui_at_rect(screen_rect, |ui| {
                        ui.add_sized(screen_rect.size(), egui::Image::new(texture));
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No processed image to display");
                    });
                }
            }
        });

        // Overlay controls on top of the image
        egui::Area::new("top_overlay")
            .fixed_pos(egui::pos2(10.0, 10.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.visuals_mut().window_fill = egui::Color32::from_black_alpha(180);
                    egui::Frame::window(&ui.style()).show(ui, |ui| {
                        ui.heading("Pixel Sorter");
                        ui.add_space(5.0);
                        
                        if self.iteration_counter > 0 {
                            ui.label(format!("Edit #{}", self.iteration_counter));
                        }
                        
                        if self.is_processing {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Processing...");
                            });
                        } else {
                            ui.label(&self.status_message);
                        }
                    });
                });
            });

        // Bottom overlay with main controls
        let screen_rect = ctx.screen_rect();
        egui::Area::new("bottom_controls")
            .fixed_pos(egui::pos2(10.0, screen_rect.height() - 150.0))
            .show(ctx, |ui| {
                ui.visuals_mut().window_fill = egui::Color32::from_black_alpha(180);
                egui::Frame::window(&ui.style()).show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Main action buttons
                        ui.horizontal(|ui| {
                            if self.preview_mode {
                                let capture_button = egui::Button::new("Capture & Sort").min_size([120.0, 40.0].into());
                                if ui.add_enabled(!self.is_processing, capture_button).clicked() {
                                    self.capture_and_sort(ctx);
                                }
                                

                            } else {
                                if ui.button("New Photo").clicked() {
                                    self.start_new_photo_session();
                                }
                                
                                if ui.button("Save & Continue").clicked() {
                                    self.save_and_continue_iteration(ctx);
                                }
                            }
                            
                            if ui.button("Load Image").clicked() {
                                self.load_image(ctx);
                            }
                            
                            if ui.button("Save to USB").clicked() {
                                match self.copy_to_usb() {
                                    Ok(()) => {
                                        self.status_message = "Successfully copied to USB!".to_string();
                                    }
                                    Err(e) => {
                                        self.status_message = format!("USB copy failed: {}", e);
                                    }
                                }
                            }
                        });
                        
                        ui.add_space(10.0);
                        
                        // Algorithm and parameters
                        ui.horizontal(|ui| {
                            ui.label("Algorithm:");
                            for &algorithm in SortingAlgorithm::all() {
                                if ui.radio_value(&mut self.current_algorithm, algorithm, algorithm.name()).clicked() {
                                    if !self.preview_mode {
                                        self.apply_pixel_sort(ctx);
                                    }
                                }
                            }
                            
                            ui.add_space(10.0);
                            ui.label(format!("Threshold: {:.0}", self.sorting_params.threshold));
                            let threshold_changed = ui.add(
                                egui::Slider::new(&mut self.sorting_params.threshold, 0.0..=255.0)
                                    .step_by(1.0)
                                    .show_value(false)
                            ).changed();

                            if threshold_changed && !self.is_processing && !self.preview_mode {
                                self.apply_pixel_sort(ctx);
                            }
                        });
                    });
                });
            });

        // Handle keyboard input for GPIO simulation
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

        // Request continuous repaints when in camera preview mode for live feed
        if self.preview_mode && self.camera_controller.is_some() {
            ctx.request_repaint();
        }

        // Only request continuous repaints when in preview mode
        if self.preview_mode && self.camera_controller.is_some() {
            ctx.request_repaint();
        }
    }
}

impl PixelSorterApp {
    // New methods for the redesigned workflow
    fn capture_and_sort(&mut self, ctx: &egui::Context) {
        if let Some(ref camera) = self.camera_controller {
            self.is_processing = true;
            self.status_message = "Capturing image...".to_string();
            
            let capture_result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let camera_lock = camera.write().await;
                    camera_lock.capture_snapshot()
                })
            });

            match capture_result {
                Ok(captured_image) => {
                    self.original_image = Some(captured_image);
                    self.preview_mode = false; // Switch to editing mode
                    // Clear camera texture to stop live updates during editing
                    self.camera_texture = None;
                    self.camera_image_data = None;
                    self.last_camera_update = None;
                    self.apply_pixel_sort(ctx);
                }
                Err(e) => {
                    self.is_processing = false;
                    self.status_message = format!("Capture failed: {}", e);
                }
            }
        }
    }

    fn apply_pixel_sort(&mut self, ctx: &egui::Context) {
        if let Some(ref original) = self.original_image.clone() {
            self.is_processing = true;
            self.status_message = format!("Applying {} sorting...", self.current_algorithm.name());
            
            let algorithm = self.current_algorithm;
            let params = self.sorting_params.clone();
            let pixel_sorter = Arc::clone(&self.pixel_sorter);
            let image = original.clone();

            // Synchronous processing for now
            match pixel_sorter.sort_pixels(&image, algorithm, &params) {
                Ok(sorted_image) => {
                    self.processed_image = Some(sorted_image.clone());
                    self.create_processed_texture(ctx, sorted_image.clone());
                    
                    self.is_processing = false;
                    self.status_message = "Processing complete!".to_string();
                }
                Err(e) => {
                    self.is_processing = false;
                    self.status_message = format!("Processing failed: {}", e);
                }
            }
        }
    }

    // Existing helper methods
    fn process_image(&mut self, ctx: &egui::Context) {
        if let Some(ref original) = self.original_image.clone() {
            self.is_processing = true;
            self.status_message = format!("Applying {} sorting...", self.current_algorithm.name());
            
            let algorithm = self.current_algorithm;
            let params = self.sorting_params.clone();
            let pixel_sorter = Arc::clone(&self.pixel_sorter);
            let image = original.clone();

            // Synchronous processing for now
            match pixel_sorter.sort_pixels(&image, algorithm, &params) {
                Ok(sorted_image) => {
                    self.processed_image = Some(sorted_image.clone());
                    self.create_processed_texture(ctx, sorted_image);
                    self.is_processing = false;
                    self.status_message = "Processing complete!".to_string();
                }
                Err(e) => {
                    self.is_processing = false;
                    self.status_message = format!("Processing failed: {}", e);
                }
            }
        }
    }

    fn load_image(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("images", &["png", "jpg", "jpeg", "gif", "bmp", "ico", "tiff", "webp", "pnm", "dds", "tga"])
            .pick_file() 
        {
            match image::open(&path) {
                Ok(img) => {
                    let rgb_image = img.to_rgb8();
                    self.original_image = Some(rgb_image.clone());
                    self.preview_mode = false; // Switch to editing mode when loading image
                    self.process_image(ctx);
                    self.status_message = format!("Loaded: {}", path.display());
                }
                Err(e) => {
                    self.status_message = format!("Failed to load image: {}", e);
                }
            }
        }
    }

    fn save_image(&mut self) {
        if let Some(ref processed) = self.processed_image {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("pixel_sorted.png")
                .add_filter("PNG", &["png"])
                .save_file()
            {
                match processed.save(&path) {
                    Ok(_) => {
                        self.status_message = format!("Saved: {}", path.display());
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to save: {}", e);
                    }
                }
            }
        } else {
            self.status_message = "No processed image to save".to_string();
        }
    }

    fn create_camera_texture(&mut self, ctx: &egui::Context, image: image::RgbImage) {
        let size = [image.width() as usize, image.height() as usize];
        let pixels = image.as_flat_samples();
        
        let color_image = egui::ColorImage::from_rgb(size, pixels.as_slice());
        let texture_name = format!("camera_preview_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
        
        let texture = ctx.load_texture(texture_name, color_image, egui::TextureOptions::LINEAR);
        self.camera_texture = Some(texture);
    }

    fn create_processed_texture(&mut self, ctx: &egui::Context, image: image::RgbImage) {
        let size = [image.width() as usize, image.height() as usize];
        let pixels = image.as_flat_samples();
        
        let color_image = egui::ColorImage::from_rgb(size, pixels.as_slice());
        let texture_name = format!("processed_image_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
        
        let texture = ctx.load_texture(texture_name, color_image, egui::TextureOptions::LINEAR);
        self.processed_texture = Some(texture);
    }

    fn on_button_press(&mut self, button: u8, ctx: &egui::Context) {
        match button {
            1 => {
                self.load_image(ctx);
            }
            2 => {
                if self.camera_controller.is_some() {
                    if self.preview_mode {
                        self.capture_and_sort(ctx);
                    } else {
                        self.preview_mode = true;
                        self.status_message = "Live preview active".to_string();
                    }
                } else {
                    // No camera - cycle algorithm
                    let current_idx = SortingAlgorithm::all().iter().position(|&x| std::mem::discriminant(&x) == std::mem::discriminant(&self.current_algorithm)).unwrap_or(0);
                    let next_idx = (current_idx + 1) % SortingAlgorithm::all().len();
                    self.current_algorithm = SortingAlgorithm::all()[next_idx];
                    self.process_image(ctx);
                }
            }
            3 => {
                if self.camera_controller.is_some() {
                    // Cycle algorithm when camera available
                    let current_idx = SortingAlgorithm::all().iter().position(|&x| std::mem::discriminant(&x) == std::mem::discriminant(&self.current_algorithm)).unwrap_or(0);
                    let next_idx = (current_idx + 1) % SortingAlgorithm::all().len();
                    self.current_algorithm = SortingAlgorithm::all()[next_idx];
                    if !self.preview_mode {
                        self.process_image(ctx);
                    }
                } else {
                    // Increase threshold when no camera
                    self.sorting_params.threshold = (self.sorting_params.threshold + 10.0).min(255.0);
                    self.process_image(ctx);
                }
            }
            4 => {
                if self.camera_controller.is_some() {
                    // Increase threshold when camera available
                    self.sorting_params.threshold = (self.sorting_params.threshold + 10.0).min(255.0);
                    if !self.preview_mode {
                        self.process_image(ctx);
                    }
                } else {
                    // Decrease threshold when no camera
                    self.sorting_params.threshold = (self.sorting_params.threshold - 10.0).max(0.0);
                    self.process_image(ctx);
                }
            }
            5 => {
                if self.camera_controller.is_some() {
                    // Decrease threshold when camera available
                    self.sorting_params.threshold = (self.sorting_params.threshold - 10.0).max(0.0);
                    if !self.preview_mode {
                        self.process_image(ctx);
                    }
                } else {
                    // Save when no camera
                    self.save_image();
                }
            }
            6 => {
                if self.camera_controller.is_some() {
                    self.save_image();
                }
            }
            _ => {}
        }
        
        self.status_message = format!("Button {} pressed - {}", button, self.current_algorithm.name());
    }

    fn auto_save_image(&mut self, image: &image::RgbImage, algorithm: &SortingAlgorithm) -> Result<PathBuf, Box<dyn std::error::Error>> {
        // Create session folder if this is the first save
        if self.current_session_folder.is_none() {
            let now: DateTime<Local> = Local::now();
            let session_folder = format!("session_{}", now.format("%Y%m%d_%H%M%S"));
            self.current_session_folder = Some(session_folder.clone());
            self.iteration_counter = 0;
        }
        
        // Create session directory
        let session_dir = PathBuf::from("sorted_images").join(self.current_session_folder.as_ref().unwrap());
        std::fs::create_dir_all(&session_dir)?;
        
        // Increment iteration counter for this save
        self.iteration_counter += 1;
        
        // Generate iteration-based filename
        let filename = format!("edit_{:03}_{}.png", 
            self.iteration_counter,
            algorithm.name().to_lowercase()
        );
        
        let save_path = session_dir.join(filename);
        image.save(&save_path)?;
        
        // Return the path for potential loading in next iteration
        Ok(save_path)
    }

    fn copy_to_usb(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Find USB drives (looking for common mount points on Linux/Pi)
        let usb_paths = [
            "/media/pi", // Pi OS default
            "/media", // Generic Linux
            "/mnt", // Manual mounts
        ];

        let mut usb_found = false;
        for base_path in &usb_paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let usb_path = entry.path();
                        if usb_path.is_dir() {
                            // Try to copy sorted_images folder to USB
                            let dest_path = usb_path.join("sorted_images");
                            if let Ok(()) = self.copy_directory(PathBuf::from("sorted_images"), dest_path.clone()) {
                                println!("Successfully copied to USB: {}", dest_path.display());
                                usb_found = true;
                                break;
                            }
                        }
                    }
                }
                if usb_found { break; }
            }
        }

        if !usb_found {
            return Err("No USB drive found or copy failed".into());
        }
        
        Ok(())
    }

    fn copy_directory<P: AsRef<std::path::Path>>(&self, src: P, dst: P) -> Result<(), Box<dyn std::error::Error>> {
        let src = src.as_ref();
        let dst = dst.as_ref();
        
        if !src.exists() {
            return Err("Source directory does not exist".into());
        }

        std::fs::create_dir_all(dst)?;

        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_file() {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    fn start_new_photo_session(&mut self) {
        // Reset session state
        self.iteration_counter = 0;
        self.current_session_folder = None;
        self.original_image = None;
        self.processed_image = None;
        self.camera_texture = None;
        self.processed_texture = None;
        self.camera_image_data = None;
        self.last_camera_update = None; // Reset camera timer to immediately start fresh
        self.preview_mode = true;
        self.status_message = "Live preview reactivated - Press button to capture!".to_string();
    }

    fn load_last_iteration_as_source(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref session_folder) = self.current_session_folder {
            if self.iteration_counter > 0 {
                // Load the last saved iteration as the new source
                let session_dir = PathBuf::from("sorted_images").join(session_folder);
                
                // Find the last saved file
                let iteration_prefix = format!("edit_{:03}_", self.iteration_counter);
                for entry in std::fs::read_dir(&session_dir)? {
                    let entry = entry?;
                    let filename = entry.file_name().to_string_lossy().to_string();
                    if filename.starts_with(&iteration_prefix) {
                        // Load this image as the new original
                        let image_path = entry.path();
                        match image::open(&image_path) {
                            Ok(img) => {
                                let rgb_image = img.to_rgb8();
                                self.original_image = Some(rgb_image);
                                return Ok(());
                            }
                            Err(e) => return Err(e.into()),
                        }
                    }
                }
            }
        }
        Err("No previous iteration found to load".into())
    }

    fn save_and_continue_iteration(&mut self, ctx: &egui::Context) {
        if let Some(ref processed) = self.processed_image.clone() {
            // Extract algorithm to avoid borrow conflict
            let algorithm = self.current_algorithm;
            // Save the current iteration using the existing auto-save system
            match self.auto_save_image(&processed, &algorithm) {
                Ok(_saved_path) => {
                    // Load the saved image as the new source for next iteration
                    if let Ok(()) = self.load_last_iteration_as_source() {
                        // Process the loaded image immediately for preview
                        self.apply_pixel_sort(ctx);
                        self.status_message = format!("Saved iteration {} - Ready for next edit", self.iteration_counter);
                    } else {
                        self.status_message = "Save successful but couldn't load for iteration".to_string();
                    }
                }
                Err(_) => {
                    self.status_message = "Failed to save iteration".to_string();
                }
            }
        } else {
            self.status_message = "No processed image to save".to_string();
        }
    }
}
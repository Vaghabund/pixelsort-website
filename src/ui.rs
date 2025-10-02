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
    // Crop functionality
    pub crop_mode: bool,
    pub zoom_level: f32,
    pub crop_rect: Option<egui::Rect>,
    pub pan_offset: egui::Vec2,
    pub selection_start: Option<egui::Pos2>,
    pub exit_requested: bool,
}

impl PixelSorterApp {
    pub fn new(
        pixel_sorter: Arc<PixelSorter>,
        image_processor: Arc<RwLock<ImageProcessor>>,
        gpio_controller: Option<Arc<RwLock<GpioController>>>,
        camera_controller: Option<Arc<RwLock<CameraController>>>,
        config: Config,
    ) -> Self {
        // Start camera streaming immediately if camera is available
        if let Some(ref camera) = camera_controller {
            if let Ok(mut camera_lock) = camera.try_write() {
                let _ = camera_lock.start_streaming();
            }
        }

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
            // Crop functionality
            crop_mode: false,
            zoom_level: 1.0,
            crop_rect: None,
            pan_offset: egui::Vec2::ZERO,
            selection_start: None,
            exit_requested: false,
        }
    }

    // Camera preview is now handled directly in the main update loop with throttling
}

impl eframe::App for PixelSorterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) -> bool {
        // Check if exit was requested
        if self.exit_requested {
            return false;
        }

        // Camera preview is now handled directly in the update loop below

        // High-performance 30 FPS camera updates - eliminate all bottlenecks
        if self.preview_mode && self.camera_controller.is_some() && !self.is_processing {
            let now = Instant::now();
            let should_update = match self.last_camera_update {
                None => true,
                Some(last) => now.duration_since(last) >= std::time::Duration::from_millis(33), // 30 FPS target
            };

            if should_update {
                // Clone the camera controller reference to avoid borrow conflicts
                if let Some(camera) = self.camera_controller.clone() {
                    // Try to get camera lock without blocking the UI
                    if let Ok(mut camera_lock) = camera.try_write() {
                        // Use the existing synchronous method - NO CLONING
                        match camera_lock.get_fast_preview_image() {
                            Ok(preview_image) => {
                                // Direct texture update without storing intermediate image
                                self.update_camera_texture(ctx, &preview_image);
                                self.last_camera_update = Some(now);
                            }
                            Err(_) => {
                                // Skip this frame if camera is busy
                            }
                        }
                    }
                    // If camera is locked, skip this frame - don't block UI
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
                // Show processed image with zoom and crop support
                if let Some(texture) = &self.processed_texture {
                    ui.allocate_ui_at_rect(screen_rect, |ui| {
                        // Handle mouse interactions for crop selection
                        let response = ui.interact(screen_rect, egui::Id::new("image_interaction"), egui::Sense::click_and_drag());

                        if self.crop_mode {
                            // Handle crop selection
                            if response.drag_started() {
                                self.selection_start = Some(response.interact_pointer_pos().unwrap_or_default());
                                self.crop_rect = None;
                            } else if response.dragged() {
                                if let Some(start) = self.selection_start {
                                    if let Some(current) = response.interact_pointer_pos() {
                                        let rect = egui::Rect::from_two_pos(start, current);
                                        self.crop_rect = Some(rect);
                                    }
                                }
                            }
                        } else {
                            // Handle panning when zoomed in
                            if response.dragged() && self.zoom_level > 1.0 {
                                self.pan_offset += response.drag_delta();
                            }
                        }

                        // Handle zoom with mouse wheel
                        let scroll_delta = ui.input(|i| i.scroll_delta);
                        if scroll_delta.y != 0.0 {
                            if scroll_delta.y > 0.0 && self.zoom_level < 5.0 {
                                self.zoom_level *= 1.05;
                            } else if scroll_delta.y < 0.0 && self.zoom_level > 0.5 {
                                self.zoom_level /= 1.05;
                            }
                        }

                        // Calculate image display parameters
                        let image_size = texture.size_vec2();
                        let scaled_size = image_size * self.zoom_level;
                        let center = screen_rect.center();

                        // Calculate image position with pan offset
                        let image_rect = egui::Rect::from_center_size(
                            center + self.pan_offset,
                            scaled_size
                        );

                        // Display the zoomed/panned image
                        let image = egui::Image::new(texture)
                            .uv(egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)));

                        ui.put(image_rect, image);

                        // Draw crop rectangle overlay
                        if let Some(crop_rect) = self.crop_rect {
                            let stroke = egui::Stroke::new(2.0, egui::Color32::RED);
                            ui.painter().rect_stroke(crop_rect, 0.0, stroke);

                            // Draw semi-transparent overlay outside crop area
                            let crop_painter = ui.painter();
                            let outside_color = egui::Color32::from_black_alpha(120);

                            // Top area
                            if crop_rect.min.y > screen_rect.min.y {
                                let top_rect = egui::Rect::from_min_max(
                                    screen_rect.min,
                                    egui::pos2(screen_rect.max.x, crop_rect.min.y)
                                );
                                crop_painter.rect_filled(top_rect, 0.0, outside_color);
                            }

                            // Bottom area
                            if crop_rect.max.y < screen_rect.max.y {
                                let bottom_rect = egui::Rect::from_min_max(
                                    egui::pos2(screen_rect.min.x, crop_rect.max.y),
                                    screen_rect.max
                                );
                                crop_painter.rect_filled(bottom_rect, 0.0, outside_color);
                            }

                            // Left area
                            if crop_rect.min.x > screen_rect.min.x {
                                let left_rect = egui::Rect::from_min_max(
                                    egui::pos2(screen_rect.min.x, crop_rect.min.y),
                                    egui::pos2(crop_rect.min.x, crop_rect.max.y)
                                );
                                crop_painter.rect_filled(left_rect, 0.0, outside_color);
                            }

                            // Right area
                            if crop_rect.max.x < screen_rect.max.x {
                                let right_rect = egui::Rect::from_min_max(
                                    egui::pos2(crop_rect.max.x, crop_rect.min.y),
                                    egui::pos2(screen_rect.max.x, crop_rect.max.y)
                                );
                                crop_painter.rect_filled(right_rect, 0.0, outside_color);
                            }
                        }
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

        // Bottom overlay with context-sensitive controls
        let screen_rect = ctx.screen_rect();
        egui::Area::new("bottom_controls")
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(10.0, -10.0))
            .show(ctx, |ui| {
                ui.visuals_mut().window_fill = egui::Color32::from_black_alpha(180);
                egui::Frame::window(&ui.style()).show(ui, |ui| {
                    if self.preview_mode {
                        // Camera Live View Layout
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                // Take Picture button
                                let capture_button = egui::Button::new("Take Picture").min_size([140.0, 40.0].into());
                                if ui.add_enabled(!self.is_processing, capture_button).clicked() {
                                    self.capture_and_sort(ctx);
                                }

                                ui.separator();

                                // Load Image button
                                if ui.button("Load Image").clicked() {
                                    self.load_image(ctx);
                                }

                                ui.separator();

                                // Exit button
                                if ui.button("Exit").clicked() {
                                    self.exit_requested = true;
                                }
                            });
                        });
                    } else {
                        // Image Editing Layout
                        ui.vertical(|ui| {
                            // Algorithm and parameters
                            ui.horizontal(|ui| {
                                ui.label("Algorithm:");
                                egui::ComboBox::from_label("")
                                    .selected_text(self.current_algorithm.name())
                                    .show_ui(ui, |ui| {
                                        for &algorithm in SortingAlgorithm::all() {
                                            if ui.selectable_value(&mut self.current_algorithm, algorithm, algorithm.name()).clicked() {
                                                self.apply_pixel_sort(ctx);
                                            }
                                        }
                                    });

                                ui.add_space(15.0);
                                ui.label(format!("Threshold: {:.0}", self.sorting_params.threshold));
                                let threshold_changed = ui.add(
                                    egui::Slider::new(&mut self.sorting_params.threshold, 0.0..=255.0)
                                        .step_by(1.0)
                                        .show_value(false)
                                ).changed();

                                if threshold_changed && !self.is_processing {
                                    self.apply_pixel_sort(ctx);
                                }
                            });

                            ui.add_space(10.0);

                            // Crop controls
                            ui.horizontal(|ui| {
                                ui.label("Zoom:");
                                if ui.button("Zoom In").clicked() && self.zoom_level < 5.0 {
                                    self.zoom_level *= 1.2;
                                }
                                if ui.button("Zoom Out").clicked() && self.zoom_level > 0.5 {
                                    self.zoom_level /= 1.2;
                                }
                                ui.label(format!("{:.1}x", self.zoom_level));

                                ui.separator();

                                if ui.button(if self.crop_mode { "Cancel Crop" } else { "Select Crop" }).clicked() {
                                    self.crop_mode = !self.crop_mode;
                                    if !self.crop_mode {
                                        self.crop_rect = None;
                                        self.selection_start = None;
                                    }
                                }

                                if self.crop_mode && self.crop_rect.is_some() {
                                    ui.separator();
                                    if ui.button("Apply Crop").clicked() {
                                        self.apply_crop_and_sort(ctx);
                                    }
                                }
                            });

                            ui.add_space(10.0);

                            // Action buttons
                            ui.horizontal(|ui| {
                                // Save & Continue button
                                if ui.button("Save & Continue").clicked() {
                                    self.save_and_continue_iteration(ctx);
                                }

                                ui.separator();

                                // Back to Camera button
                                if ui.button("Back to Camera").clicked() {
                                    self.start_new_photo_session();
                                }

                                ui.separator();

                                // Export to USB button
                                if ui.button("Export to USB").clicked() {
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
                        });
                    }
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

        // High-performance 30 FPS repaints for smooth camera feed
        if self.preview_mode && self.camera_controller.is_some() && !self.is_processing {
            ctx.request_repaint_after(std::time::Duration::from_millis(33)); // 30 FPS
        }

        true
    }
}

impl PixelSorterApp {
    // New methods for the redesigned workflow
    fn capture_and_sort(&mut self, ctx: &egui::Context) {
        // Clone the camera controller reference to avoid borrow conflicts
        if let Some(camera) = self.camera_controller.clone() {
            self.is_processing = true;
            self.status_message = "Capturing image...".to_string();

            // Stop streaming during capture for better image quality
            if let Ok(mut camera_lock) = camera.try_write() {
                camera_lock.stop_streaming();

                match camera_lock.capture_snapshot() {
                    Ok(captured_image) => {
                        self.original_image = Some(captured_image);
                        self.preview_mode = false; // Switch to editing mode
                        // Clear camera texture to stop live updates during editing
                        self.camera_texture = None;
                        self.last_camera_update = None;
                        self.apply_pixel_sort(ctx);
                    }
                    Err(e) => {
                        self.is_processing = false;
                        self.status_message = format!("Capture failed: {}", e);
                        // Restart streaming on failure
                        let _ = camera_lock.start_streaming();
                    }
                }
            } else {
                // Camera is busy, try again later
                self.is_processing = false;
                self.status_message = "Camera busy, please try again".to_string();
            }
        }
    }

    fn apply_pixel_sort(&mut self, ctx: &egui::Context) {
        if let Some(ref original) = self.original_image {
            self.is_processing = true;
            self.status_message = format!("Applying {} sorting...", self.current_algorithm.name());
            
            let algorithm = self.current_algorithm;
            let params = self.sorting_params.clone();
            let pixel_sorter = Arc::clone(&self.pixel_sorter);

            // Avoid unnecessary cloning - use reference
            match pixel_sorter.sort_pixels(original, algorithm, &params) {
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

    // High-performance texture update that avoids unnecessary copying
    fn update_camera_texture(&mut self, ctx: &egui::Context, image: &image::RgbImage) {
        let size = [image.width() as usize, image.height() as usize];
        let pixels = image.as_flat_samples();
        
        let color_image = egui::ColorImage::from_rgb(size, pixels.as_slice());
        
        // Reuse existing texture - optimized for 30 FPS updates
        match &mut self.camera_texture {
            Some(texture) => {
                // Direct update - no allocation
                texture.set(color_image, egui::TextureOptions::LINEAR);
            }
            None => {
                // First time only
                let texture = ctx.load_texture("camera_preview", color_image, egui::TextureOptions::LINEAR);
                self.camera_texture = Some(texture);
            }
        }
    }

    fn create_processed_texture(&mut self, ctx: &egui::Context, image: image::RgbImage) {
        let size = [image.width() as usize, image.height() as usize];
        let pixels = image.as_flat_samples();
        
        let color_image = egui::ColorImage::from_rgb(size, pixels.as_slice());
        
        // Reuse existing texture if available to reduce memory allocations
        match &mut self.processed_texture {
            Some(texture) => {
                // Update existing texture instead of creating new one
                texture.set(color_image, egui::TextureOptions::LINEAR);
            }
            None => {
                let texture = ctx.load_texture("processed_image", color_image, egui::TextureOptions::LINEAR);
                self.processed_texture = Some(texture);
            }
        }
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
        self.last_camera_update = None; // Reset camera timer to immediately start fresh
        self.preview_mode = true;

        // Restart camera streaming for new session
        if let Some(ref camera) = self.camera_controller {
            if let Ok(mut camera_lock) = camera.try_write() {
                let _ = camera_lock.start_streaming();
            }
        }

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

    fn apply_crop_and_sort(&mut self, ctx: &egui::Context) {
        if let (Some(ref original), Some(crop_rect)) = (&self.original_image, self.crop_rect) {
            self.is_processing = true;
            self.status_message = format!("Cropping and applying {} sorting...", self.current_algorithm.name());

            // Get screen and image dimensions for coordinate conversion
            let screen_rect = ctx.screen_rect();
            let image_size = original.dimensions();

            // Convert screen coordinates to image coordinates
            let image_rect = egui::Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(image_size.0 as f32, image_size.1 as f32)
            );

            // Calculate the transformation from screen to image coordinates
            let scale_x = image_size.0 as f32 / screen_rect.width();
            let scale_y = image_size.1 as f32 / screen_rect.height();

            // Adjust for zoom and pan
            let zoom_center = screen_rect.center();
            let image_center = image_rect.center();

            // Convert crop rectangle to image coordinates
            let crop_min_x = ((crop_rect.min.x - zoom_center.x) * scale_x / self.zoom_level + image_center.x - self.pan_offset.x * scale_x / self.zoom_level).max(0.0) as u32;
            let crop_min_y = ((crop_rect.min.y - zoom_center.y) * scale_y / self.zoom_level + image_center.y - self.pan_offset.y * scale_y / self.zoom_level).max(0.0) as u32;
            let crop_max_x = ((crop_rect.max.x - zoom_center.x) * scale_x / self.zoom_level + image_center.x - self.pan_offset.x * scale_x / self.zoom_level).min(image_size.0 as f32) as u32;
            let crop_max_y = ((crop_rect.max.y - zoom_center.y) * scale_y / self.zoom_level + image_center.y - self.pan_offset.y * scale_y / self.zoom_level).min(image_size.1 as f32) as u32;

            // Ensure valid crop dimensions
            let crop_width = crop_max_x.saturating_sub(crop_min_x);
            let crop_height = crop_max_y.saturating_sub(crop_min_y);

            if crop_width > 0 && crop_height > 0 {
                // Create cropped image
                let mut cropped = image::RgbImage::new(crop_width, crop_height);

                for y in 0..crop_height {
                    for x in 0..crop_width {
                        let src_x = crop_min_x + x;
                        let src_y = crop_min_y + y;
                        if src_x < image_size.0 && src_y < image_size.1 {
                            let pixel = original.get_pixel(src_x, src_y);
                            cropped.put_pixel(x, y, *pixel);
                        }
                    }
                }

                // Apply pixel sorting to the cropped region
                let algorithm = self.current_algorithm;
                let params = self.sorting_params.clone();
                let pixel_sorter = Arc::clone(&self.pixel_sorter);

                match pixel_sorter.sort_pixels(&cropped, algorithm, &params) {
                    Ok(sorted_cropped) => {
                        // Make the sorted cropped region the new full image
                        self.original_image = Some(sorted_cropped.clone());
                        self.processed_image = Some(sorted_cropped.clone());
                        self.create_processed_texture(ctx, sorted_cropped);

                        // Exit crop mode and reset zoom/pan
                        self.crop_mode = false;
                        self.crop_rect = None;
                        self.selection_start = None;
                        self.zoom_level = 1.0;
                        self.pan_offset = egui::Vec2::ZERO;

                        self.is_processing = false;
                        self.status_message = "Crop processed successfully!".to_string();
                    }
                    Err(e) => {
                        self.is_processing = false;
                        self.status_message = format!("Processing failed: {}", e);
                    }
                }
            } else {
                self.is_processing = false;
                self.status_message = "Invalid crop selection".to_string();
            }
        } else {
            self.status_message = "No image or crop selection available".to_string();
        }
    }
}
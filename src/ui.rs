use std::sync::Arc;
use std::path::PathBuf;
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
    pub current_texture: Option<egui::TextureHandle>,
    pub pixel_sorter: Arc<PixelSorter>,
    pub current_algorithm: SortingAlgorithm,
    pub sorting_params: SortingParameters,
    pub is_processing: bool,
    pub status_message: String,
    pub camera_controller: Option<Arc<RwLock<CameraController>>>,
    pub gpio_controller: Option<Arc<RwLock<GpioController>>>,
    pub image_processor: Arc<RwLock<ImageProcessor>>,
    pub config: Config,
    pub preview_mode: bool,
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
            current_texture: None,
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
        }
    }

    fn start_camera_preview(&self, ctx: &egui::Context) {
        if let Some(ref camera) = self.camera_controller {
            let camera = Arc::clone(camera);
            let ctx_clone = ctx.clone();
            
            tokio::spawn(async move {
                loop {
                    let mut camera_lock = camera.write().await;
                    match camera_lock.get_preview_image() {
                        Ok(_preview) => {
                            ctx_clone.request_repaint();
                        }
                        Err(e) => {
                            println!("Preview update failed: {}", e);
                        }
                    }
                    drop(camera_lock);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            });
        }
    }
}

impl eframe::App for PixelSorterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Start camera preview on first update if we have a camera
        if self.camera_controller.is_some() && self.current_texture.is_none() && self.preview_mode {
            self.start_camera_preview(ctx);
        }

        // Update camera preview texture if in preview mode
        if self.preview_mode && self.camera_controller.is_some() {
            if let Some(ref camera) = self.camera_controller {
                let preview_result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let mut camera_lock = camera.write().await;
                        camera_lock.get_preview_image()
                    })
                });

                match preview_result {
                    Ok(preview_image) => {
                        self.create_texture_from_image(ctx, preview_image);
                    }
                    Err(e) => {
                        println!("Failed to get preview: {}", e);
                    }
                }
            }
        }

        // Main UI Layout - Side panel for controls, Central panel for image
        egui::SidePanel::left("controls")
            .min_width(250.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.heading("Pixel Sorter");
                    ui.add_space(20.0);

                    // Camera/Mode controls
                    if self.preview_mode {
                        let capture_button = egui::Button::new("📸 Capture & Sort").min_size([200.0, 50.0].into());
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

                    if ui.add_sized([200.0, 50.0], egui::Button::new("📀 Save to USB")).clicked() {
                        match self.copy_to_usb() {
                            Ok(()) => {
                                self.status_message = "Successfully copied to USB!".to_string();
                            }
                            Err(e) => {
                                self.status_message = format!("USB copy failed: {}", e);
                            }
                        }
                    }

                    ui.add_space(10.0);

                    if ui.add_sized([200.0, 50.0], egui::Button::new("⛶ Force Fullscreen")).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                    }

                    if ui.add_sized([200.0, 50.0], egui::Button::new("❌ Exit")).clicked() {
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
                            if !self.preview_mode {
                                self.apply_pixel_sort(ctx);
                            }
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

                    // Auto-process when parameters change
                    if threshold_changed && !self.is_processing && !self.preview_mode {
                        self.apply_pixel_sort(ctx);
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
                            ui.label("2: Capture & Sort 📸");
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

        // Central panel for image display
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                if self.preview_mode {
                    // Show camera preview or prompt
                    if let Some(ref _camera) = self.camera_controller {
                        if let Some(texture) = &self.current_texture {
                            let available_size = ui.available_size();
                            let texture_size = texture.size_vec2();
                            let scale = (available_size.x / texture_size.x).min(available_size.y / texture_size.y).min(1.0);
                            let display_size = texture_size * scale;

                            println!("DEBUG - Texture render:");
                            println!("  Available size: {:?}", available_size);
                            println!("  Texture size: {:?}", texture_size);
                            println!("  Scale: {}", scale);
                            println!("  Display size: {:?}", display_size);

                            ui.add(
                                egui::Image::new(texture)
                                    .fit_to_exact_size(display_size)
                            );
                        } else {
                            ui.label("Initializing camera...");
                        }
                    } else {
                        ui.label("No camera available - Load an image to begin");
                    }
                } else {
                    // Show processed image
                    if let Some(texture) = &self.current_texture {
                        let available_size = ui.available_size();
                        let texture_size = texture.size_vec2();
                        let scale = (available_size.x / texture_size.x).min(available_size.y / texture_size.y).min(1.0);
                        let display_size = texture_size * scale;

                        ui.add(
                            egui::Image::new(texture)
                                .fit_to_exact_size(display_size)
                        );
                    } else {
                        ui.label("No processed image to display");
                    }
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

        // Request repaint for smooth updates
        ctx.request_repaint();
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
                    self.apply_pixel_sort(ctx);
                }
                Err(e) => {
                    self.is_processing = false;
                    self.status_message = format!("Capture failed: {}", e);
                }
            }
        }
    }

    fn save_and_return_to_camera(&mut self) {
        // TODO: Implement actual save functionality
        self.preview_mode = true;
        self.status_message = "Returned to camera preview".to_string();
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
                    self.create_texture_from_image(ctx, sorted_image.clone());
                    
                    // Auto-save the processed image
                    if let Err(e) = self.auto_save_image(&sorted_image, &algorithm) {
                        println!("Auto-save failed: {}", e);
                    }
                    
                    self.is_processing = false;
                    self.status_message = "Processing complete - Image auto-saved!".to_string();
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
                    self.create_texture_from_image(ctx, sorted_image);
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

    fn create_texture_from_image(&mut self, ctx: &egui::Context, image: image::RgbImage) {
        let size = [image.width() as usize, image.height() as usize];
        let pixels = image.as_flat_samples();
        
        let color_image = egui::ColorImage::from_rgb(size, pixels.as_slice());
        let texture = ctx.load_texture("processed_image", color_image, egui::TextureOptions::LINEAR);
        self.current_texture = Some(texture);
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
    # Clean up old systemd temp files
    sudo find /tmp -name "systemd-private-*" -type d -mtime +1 -exec rm -rf {} +
    
    # Clean up old namespace files  
    sudo find /tmp -name "namespace-dev-*" -type f -mtime +1 -delete
    
    # Or just reboot (cleanest option)
    sudo reboot    }

    fn auto_save_image(&self, image: &image::RgbImage, algorithm: &SortingAlgorithm) -> Result<(), Box<dyn std::error::Error>> {
        // Create sorted_images directory if it doesn't exist
        let save_dir = PathBuf::from("sorted_images");
        std::fs::create_dir_all(&save_dir)?;
        
        // Generate timestamp-based filename
        let now: DateTime<Local> = Local::now();
        let filename = format!("pixelsort_{}_{}.png", 
            algorithm.name().to_lowercase(),
            now.format("%Y%m%d_%H%M%S")
        );
        
        let save_path = save_dir.join(filename);
        image.save(&save_path)?;
        
        println!("Auto-saved image to: {}", save_path.display());
        Ok(())
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
                            if let Ok(()) = self.copy_directory(PathBuf::from("sorted_images"), &dest_path) {
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
}
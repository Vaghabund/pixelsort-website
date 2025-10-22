use std::sync::Arc;
use std::time::Instant;
use eframe::egui;
use tokio::sync::RwLock;

use crate::pixel_sorter::{PixelSorter, SortingAlgorithm, SortingParameters};
use crate::camera_controller::CameraController;

// ============================================================================
// CONSTANTS FOR UI STYLING - Easy to modify
// ============================================================================
const HANDLE_SIZE: f32 = 28.0; // Bigger crop handles
const UI_PADDING: f32 = 20.0; // Padding from screen edges

// ============================================================================
// ENUMS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Input,
    Edit,
    Crop,
}

// ============================================================================
// MAIN APP STRUCT
// ============================================================================

pub struct PixelSorterApp {
    // Phase management
    pub current_phase: Phase,
    
    // Image data
    pub original_image: Option<image::RgbImage>,
    pub processed_image: Option<image::RgbImage>,
    pub camera_texture: Option<egui::TextureHandle>,
    pub processed_texture: Option<egui::TextureHandle>,
    
    // Processing
    pub pixel_sorter: Arc<PixelSorter>,
    pub current_algorithm: SortingAlgorithm,
    pub sorting_params: SortingParameters,
    pub is_processing: bool,
    
    // Camera
    pub camera_controller: Option<Arc<RwLock<CameraController>>>,
    pub last_camera_update: Option<Instant>,
    pub preview_mode: bool,
    
    // Crop state
    pub crop_rect: Option<egui::Rect>, // In image coordinates
    pub drag_state: DragState,
    
    // Session management
    pub iteration_counter: u32,
    pub current_session_folder: Option<String>,
    
    // Export status
    pub export_message: Option<String>,
    pub export_message_time: Option<Instant>,
    
    // Splash screen
    pub show_splash: bool,
    pub splash_start_time: Option<Instant>,
    pub splash_logo: Option<egui::TextureHandle>,
    
    // Exit mechanism for kiosk mode
    pub exit_tap_count: u32,
    pub exit_tap_last_time: Option<Instant>,
    
    // Other
    pub tint_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragState {
    None,
    DraggingHandle(HandlePosition),
    MovingCrop,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandlePosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

// ============================================================================
// INITIALIZATION
// ============================================================================

impl PixelSorterApp {
    pub fn new(
        pixel_sorter: Arc<PixelSorter>,
        camera_controller: Option<Arc<RwLock<CameraController>>>,
    ) -> Self {
        // Start camera streaming if available
        if let Some(ref camera) = camera_controller {
            if let Ok(mut camera_lock) = camera.try_write() {
                let _ = camera_lock.start_streaming();
            }
        }

        Self {
            current_phase: Phase::Input,
            original_image: None,
            processed_image: None,
            camera_texture: None,
            processed_texture: None,
            pixel_sorter,
            current_algorithm: SortingAlgorithm::Horizontal,
            sorting_params: SortingParameters::default(),
            is_processing: false,
            camera_controller,
            last_camera_update: None,
            preview_mode: true,
            crop_rect: None,
            drag_state: DragState::None,
            iteration_counter: 0,
            current_session_folder: None,
            export_message: None,
            export_message_time: None,
            show_splash: true,
            splash_start_time: Some(Instant::now()),
            splash_logo: None,
            exit_tap_count: 0,
            exit_tap_last_time: None,
            tint_enabled: false,
        }
    }

    fn usb_present(&self) -> bool {
        let usb_paths = ["/media/pi", "/media/usb", "/media", "/mnt/usb", "/mnt"];
        for base_path in &usb_paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let usb_path = entry.path();
                    
                    // Skip if not a directory or if it's the pi user home
                    if !usb_path.is_dir() || usb_path.to_string_lossy().contains("/home/") {
                        continue;
                    }
                    
                    // Check if we can write to this path (indicates writable USB)
                    let test_file = usb_path.join(".pixelsort_usb_check");
                    if std::fs::write(&test_file, "test").is_ok() {
                        let _ = std::fs::remove_file(&test_file);
                        return true;
                    }
                }
            }
        }
        false
    }
}

// ============================================================================
// MAIN UPDATE LOOP
// ============================================================================

impl eframe::App for PixelSorterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ESC key to exit (for debugging in kiosk mode with keyboard)
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        
        // Hidden exit area for touchscreen (top-left corner, tap 5 times within 3 seconds)
        egui::Area::new("exit_area")
            .fixed_pos(egui::pos2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let exit_button_size = egui::vec2(50.0, 50.0);
                let (_rect, response) = ui.allocate_exact_size(exit_button_size, egui::Sense::click());
                
                if response.clicked() {
                    let now = Instant::now();
                    
                    // Reset count if more than 3 seconds passed since last tap
                    if let Some(last_time) = self.exit_tap_last_time {
                        if now.duration_since(last_time).as_secs() > 3 {
                            self.exit_tap_count = 0;
                        }
                    }
                    
                    self.exit_tap_count += 1;
                    self.exit_tap_last_time = Some(now);
                    
                    // Exit after 5 taps
                    if self.exit_tap_count >= 5 {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            });
        
        // Show splash screen for 2 seconds
        if self.show_splash {
            if let Some(start_time) = self.splash_start_time {
                let elapsed = start_time.elapsed().as_secs_f32();
                if elapsed > 2.0 {
                    self.show_splash = false;
                } else {
                    self.render_splash_screen(ctx, elapsed);
                    ctx.request_repaint(); // Keep repainting for fade effect
                    return;
                }
            }
        }
        
        // Update camera preview at 30 FPS if in Input phase
        if self.current_phase == Phase::Input && !self.is_processing {
            self.update_camera_preview(ctx);
            // Request continuous repaints for smooth 30 FPS preview
            ctx.request_repaint();
        }

        // Render UI based on current phase
        self.render_ui(ctx);
    }
}

impl PixelSorterApp {
    fn update_camera_preview(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let should_update = match self.last_camera_update {
            None => true,
            Some(last) => now.duration_since(last) >= std::time::Duration::from_millis(33),
        };

        if should_update {
            if let Some(camera) = self.camera_controller.clone() {
                if let Ok(mut camera_lock) = camera.try_write() {
                    if let Ok(preview_image) = camera_lock.get_fast_preview_image() {
                        self.update_camera_texture(ctx, &preview_image);
                        self.last_camera_update = Some(now);
                    }
                }
            }
        }
    }

    fn render_splash_screen(&mut self, ctx: &egui::Context, elapsed: f32) {
        // Load logo texture if not loaded yet
        if self.splash_logo.is_none() {
            if let Ok(img) = image::open("assets/Harpy_ICON.png") {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.as_flat_samples();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                self.splash_logo = Some(ctx.load_texture("splash_logo", color_image, Default::default()));
            }
        }

        // Calculate fade alpha (fade in first 0.3s, stay visible, fade out last 0.5s)
        let alpha = if elapsed < 0.3 {
            // Fade in
            elapsed / 0.3
        } else if elapsed > 1.5 {
            // Fade out
            (2.0 - elapsed) / 0.5
        } else {
            // Fully visible
            1.0
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let screen_rect = ui.max_rect();
                
                // Black background
                ui.painter().rect_filled(
                    screen_rect,
                    0.0,
                    egui::Color32::BLACK,
                );

                // Center content
                let center = screen_rect.center();
                
                // Draw logo
                if let Some(logo_texture) = &self.splash_logo {
                    let logo_size = 256.0; // Size of the logo
                    let logo_rect = egui::Rect::from_center_size(
                        egui::pos2(center.x, center.y - 40.0),
                        egui::vec2(logo_size, logo_size),
                    );
                    
                    let tint = egui::Color32::from_white_alpha((alpha * 255.0) as u8);
                    ui.painter().image(
                        logo_texture.id(),
                        logo_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        tint,
                    );
                }

                // Draw "Harpy" text below logo
                let text = "Harpy";
                let font_id = egui::FontId::proportional(48.0);
                let text_color = egui::Color32::from_white_alpha((alpha * 255.0) as u8);
                let galley = ui.painter().layout_no_wrap(text.to_string(), font_id, text_color);
                
                let text_pos = egui::pos2(
                    center.x - galley.size().x / 2.0,
                    center.y + 120.0,
                );
                ui.painter().galley(text_pos, galley);
            });
    }

    fn render_ui(&mut self, ctx: &egui::Context) {
        // Fullscreen image with NO panels - use CentralPanel for everything
        egui::CentralPanel::default()
            .frame(egui::Frame::none()) // No frame/padding
            .show(ctx, |ui| {
                // Get full available space
                let full_rect = ui.max_rect();
                
                // Render fullscreen viewport (image fills entire window)
                self.render_viewport(ui, full_rect, ctx);
                
                // Overlay button zone at bottom using Area (floats on top)
                self.render_button_overlay(ui, ctx, full_rect);
                
                // Show export status message popup (centered, top-center)
                self.render_export_message(ctx, full_rect);
            });
    }
    
    fn render_export_message(&mut self, ctx: &egui::Context, _screen_rect: egui::Rect) {
        // Auto-hide message after 3 seconds
        if let Some(message_time) = self.export_message_time {
            if message_time.elapsed().as_secs() > 3 {
                self.export_message = None;
                self.export_message_time = None;
            }
        }
        
        if let Some(ref message) = self.export_message {
            let is_success = message.starts_with('✓');
            
            egui::Area::new("export_message")
                .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, UI_PADDING * 3.0))
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(if is_success {
                            egui::Color32::from_rgb(40, 120, 40) // Green for success
                        } else {
                            egui::Color32::from_rgb(180, 40, 40) // Red for error
                        })
                        .rounding(8.0)
                        .inner_margin(egui::Margin::symmetric(20.0, 15.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(message)
                                    .color(egui::Color32::WHITE)
                                    .size(20.0)
                            );
                        });
                });
        }
    }
}

// ============================================================================
// VIEWPORT RENDERING
// ============================================================================

impl PixelSorterApp {
    fn render_viewport(&mut self, ui: &mut egui::Ui, rect: egui::Rect, ctx: &egui::Context) {
        match self.current_phase {
            Phase::Input => self.render_input_viewport(ui, rect),
            Phase::Edit => self.render_edit_viewport(ui, rect),
            Phase::Crop => self.render_crop_viewport(ui, rect, ctx),
        }
    }

    fn render_input_viewport(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        // Draw dark grey background
        ui.painter().rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgb(40, 40, 40), // Dark grey
        );

        if let Some(texture) = &self.camera_texture {
            let image_size = texture.size_vec2();
            let display_size = fit_image_in_rect(image_size, rect.size());
            let centered_rect = center_rect_in_rect(display_size, rect);
            
            ui.allocate_ui_at_rect(centered_rect, |ui| {
                ui.add(egui::Image::new(texture).fit_to_exact_size(display_size));
            });
        } else {
            ui.allocate_ui_at_rect(rect, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("No camera available");
                });
            });
        }
    }

    fn render_edit_viewport(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        // Draw dark grey background
        ui.painter().rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgb(40, 40, 40), // Dark grey
        );

        if let Some(texture) = &self.processed_texture {
            let image_size = texture.size_vec2();
            let display_size = fit_image_in_rect(image_size, rect.size());
            let centered_rect = center_rect_in_rect(display_size, rect);
            
            ui.allocate_ui_at_rect(centered_rect, |ui| {
                ui.add(egui::Image::new(texture).fit_to_exact_size(display_size));
            });
        } else {
            ui.allocate_ui_at_rect(rect, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("No image");
                });
            });
        }
    }

    fn render_crop_viewport(&mut self, ui: &mut egui::Ui, rect: egui::Rect, ctx: &egui::Context) {
        // Draw dark grey background
        ui.painter().rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgb(40, 40, 40), // Dark grey
        );

        if let Some(texture) = &self.processed_texture {
            let image_size = texture.size_vec2();
            let display_size = fit_image_in_rect(image_size, rect.size());
            let centered_rect = center_rect_in_rect(display_size, rect);
            
            ui.allocate_ui_at_rect(centered_rect, |ui| {
                ui.add(egui::Image::new(texture).fit_to_exact_size(display_size));
            });

            // Draw overlay and crop handles
            self.render_crop_overlay(ui, centered_rect, image_size, ctx);
        }
    }

    fn render_crop_overlay(
        &mut self,
        ui: &mut egui::Ui,
        display_rect: egui::Rect,
        image_size: egui::Vec2,
        _ctx: &egui::Context,
    ) {
        // Scale factor from image to display coordinates
        let scale_x = display_rect.width() / image_size.x;
        let scale_y = display_rect.height() / image_size.y;
        let scale = scale_x.min(scale_y);

        // Initialize crop rect if needed
        if self.crop_rect.is_none() {
            let margin = 50.0;
            self.crop_rect = Some(egui::Rect::from_min_max(
                egui::pos2(margin, margin),
                egui::pos2(image_size.x - margin, image_size.y - margin),
            ));
        }

        let crop_rect = self.crop_rect.unwrap();
        
        // Convert crop rect to display coordinates
        let crop_display = egui::Rect::from_min_max(
            display_rect.min + egui::vec2(crop_rect.min.x * scale, crop_rect.min.y * scale),
            display_rect.min + egui::vec2(crop_rect.max.x * scale, crop_rect.max.y * scale),
        );

        // Handle interactions first (before borrowing painter)
        self.handle_crop_interactions(ui, crop_display, display_rect, image_size, scale);
        
        // Now borrow painter for drawing
        let painter = ui.painter();

        // Draw grey overlay outside crop area
        let grey = egui::Color32::from_black_alpha(180);
        
        // Top
        painter.rect_filled(
            egui::Rect::from_min_max(display_rect.min, egui::pos2(display_rect.max.x, crop_display.min.y)),
            0.0,
            grey,
        );
        // Bottom
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(display_rect.min.x, crop_display.max.y), display_rect.max),
            0.0,
            grey,
        );
        // Left
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(display_rect.min.x, crop_display.min.y),
                egui::pos2(crop_display.min.x, crop_display.max.y),
            ),
            0.0,
            grey,
        );
        // Right
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(crop_display.max.x, crop_display.min.y),
                egui::pos2(display_rect.max.x, crop_display.max.y),
            ),
            0.0,
            grey,
        );

        // Draw crop border
        painter.rect_stroke(crop_display, 0.0, egui::Stroke::new(3.0, egui::Color32::WHITE));

        // Draw handles
        self.draw_crop_handles(painter, crop_display);
    }

    fn handle_crop_interactions(
        &mut self,
        ui: &mut egui::Ui,
        crop_display: egui::Rect,
        display_rect: egui::Rect,
        image_size: egui::Vec2,
        scale: f32,
    ) {
        let handles = [
            (HandlePosition::TopLeft, crop_display.left_top()),
            (HandlePosition::TopRight, crop_display.right_top()),
            (HandlePosition::BottomLeft, crop_display.left_bottom()),
            (HandlePosition::BottomRight, crop_display.right_bottom()),
        ];

        // Check handle interactions
        for (handle_pos, handle_center) in handles {
            let handle_rect = egui::Rect::from_center_size(handle_center, egui::vec2(HANDLE_SIZE, HANDLE_SIZE));
            let response = ui.interact(handle_rect, ui.id().with(format!("{:?}", handle_pos)), egui::Sense::drag());
            
            if response.drag_started() {
                self.drag_state = DragState::DraggingHandle(handle_pos);
            }
            
            if response.dragged() && self.drag_state == DragState::DraggingHandle(handle_pos) {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.update_crop_rect_from_handle(handle_pos, pos, display_rect, image_size, scale);
                }
            }
        }

        // Move crop area by dragging inside
        let crop_response = ui.interact(crop_display, ui.id().with("crop_move"), egui::Sense::drag());
        
        if crop_response.drag_started() && self.drag_state == DragState::None {
            self.drag_state = DragState::MovingCrop;
        }
        
        if crop_response.dragged() && self.drag_state == DragState::MovingCrop {
            let delta = crop_response.drag_delta() / scale;
            if let Some(mut rect) = self.crop_rect {
                rect = rect.translate(delta);
                // Clamp to image bounds
                rect.min.x = rect.min.x.max(0.0);
                rect.min.y = rect.min.y.max(0.0);
                rect.max.x = rect.max.x.min(image_size.x);
                rect.max.y = rect.max.y.min(image_size.y);
                self.crop_rect = Some(rect);
            }
        }

        // Reset drag state on release
        if ui.input(|i| i.pointer.any_released()) {
            self.drag_state = DragState::None;
        }
    }

    fn update_crop_rect_from_handle(
        &mut self,
        handle: HandlePosition,
        screen_pos: egui::Pos2,
        display_rect: egui::Rect,
        image_size: egui::Vec2,
        scale: f32,
    ) {
        if let Some(mut rect) = self.crop_rect {
            // Convert screen position to image coordinates
            let image_pos = (screen_pos - display_rect.min) / scale;
            
            // Update rect based on which handle
            match handle {
                HandlePosition::TopLeft => {
                    rect.min = egui::pos2(
                        image_pos.x.max(0.0).min(rect.max.x - 10.0),
                        image_pos.y.max(0.0).min(rect.max.y - 10.0),
                    );
                }
                HandlePosition::TopRight => {
                    rect.min.y = image_pos.y.max(0.0).min(rect.max.y - 10.0);
                    rect.max.x = image_pos.x.min(image_size.x).max(rect.min.x + 10.0);
                }
                HandlePosition::BottomLeft => {
                    rect.min.x = image_pos.x.max(0.0).min(rect.max.x - 10.0);
                    rect.max.y = image_pos.y.min(image_size.y).max(rect.min.y + 10.0);
                }
                HandlePosition::BottomRight => {
                    rect.max = egui::pos2(
                        image_pos.x.min(image_size.x).max(rect.min.x + 10.0),
                        image_pos.y.min(image_size.y).max(rect.min.y + 10.0),
                    );
                }
            }

            self.crop_rect = Some(rect);
        }
    }

    fn draw_crop_handles(&self, painter: &egui::Painter, crop_display: egui::Rect) {
        let handle_color = egui::Color32::WHITE;

        // Corner handles
        let handles = [
            crop_display.left_top(),
            crop_display.right_top(),
            crop_display.left_bottom(),
            crop_display.right_bottom(),
        ];

        for center in handles {
            painter.circle_filled(center, HANDLE_SIZE / 2.0, handle_color);
            painter.circle_stroke(center, HANDLE_SIZE / 2.0, egui::Stroke::new(2.0, egui::Color32::BLACK));
        }
    }
}

// ============================================================================
// BUTTON ZONE RENDERING (OVERLAY) - UPDATED FOR CIRCULAR TOUCH UI
// ============================================================================

impl PixelSorterApp {
    fn render_button_overlay(&mut self, _ui: &mut egui::Ui, ctx: &egui::Context, screen_rect: egui::Rect) {
        // No background panel needed - buttons float directly
        match self.current_phase {
            Phase::Input => self.render_input_buttons_circular(ctx, screen_rect),
            Phase::Edit => self.render_edit_buttons_circular(ctx, screen_rect),
            Phase::Crop => self.render_crop_buttons_circular(ctx, screen_rect),
        }
    }

    // ============================================================================
    // PHASE 1: INPUT - Two circles in right bottom corner
    // ============================================================================
    fn render_input_buttons_circular(&mut self, ctx: &egui::Context, screen_rect: egui::Rect) {
        const LARGE_BUTTON_RADIUS: f32 = 100.0;  // Take Picture (larger for primary action)
        const SMALL_BUTTON_RADIUS: f32 = 50.0;   // Upload Image
        const SPACING: f32 = 20.0;
        
        // Calculate positions - right bottom corner alignment
        let large_center = egui::pos2(
            screen_rect.max.x - LARGE_BUTTON_RADIUS - SPACING,
            screen_rect.max.y - LARGE_BUTTON_RADIUS - SPACING,
        );
        
        let small_center = egui::pos2(
            screen_rect.max.x - SMALL_BUTTON_RADIUS - SPACING,
            large_center.y - LARGE_BUTTON_RADIUS - SMALL_BUTTON_RADIUS - SPACING,
        );
        
        // Draw buttons using Area widgets
        egui::Area::new("take_picture_btn")
            .fixed_pos(large_center - egui::vec2(LARGE_BUTTON_RADIUS, LARGE_BUTTON_RADIUS))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                if self.circular_button(ui, LARGE_BUTTON_RADIUS, "", "take_pic") {
                    self.capture_and_sort(ctx);
                }
            });
        
        egui::Area::new("upload_btn")
            .fixed_pos(small_center - egui::vec2(SMALL_BUTTON_RADIUS, SMALL_BUTTON_RADIUS))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                if self.circular_button(ui, SMALL_BUTTON_RADIUS, "Upload", "upload_img") {
                    self.load_image(ctx);
                }
            });
    }

    // ============================================================================
    // PHASE 2: EDIT - Horizontal sliders on right, buttons on left in two rows
    // ============================================================================
    fn render_edit_buttons_circular(&mut self, ctx: &egui::Context, screen_rect: egui::Rect) {
        const BUTTON_RADIUS: f32 = 80.0;  // Larger buttons for better touch targets
        const SLIDER_WIDTH: f32 = 60.0;
        const SLIDER_HEIGHT: f32 = 300.0;
        const SPACING: f32 = 20.0;
        
        // Right side: Horizontal sliders (side by side)
        self.render_vertical_sliders(ctx, screen_rect, SLIDER_WIDTH, SLIDER_HEIGHT, SPACING);
        
        // Left side: Buttons in two rows, aligned to left border
        // Row 1: Algorithm and Sort Mode buttons (top row) - 2 buttons
        let row1_y = screen_rect.max.y - BUTTON_RADIUS * 4.0 - SPACING * 3.0;
        
        // Algorithm button (left)
        egui::Area::new("algo_btn")
            .fixed_pos(egui::pos2(SPACING, row1_y))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                if self.circular_button(ui, BUTTON_RADIUS, self.current_algorithm.name(), "algo") {
                    self.cycle_algorithm();
                    self.apply_pixel_sort(ctx);
                }
            });
        
        // Sort Mode button (right of Algorithm)
        egui::Area::new("mode_btn")
            .fixed_pos(egui::pos2(SPACING + BUTTON_RADIUS * 2.0 + SPACING, row1_y))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                if self.circular_button(ui, BUTTON_RADIUS, self.sorting_params.sort_mode.name(), "mode") {
                    self.sorting_params.sort_mode = self.sorting_params.sort_mode.next();
                    self.apply_pixel_sort(ctx);
                }
            });
        
        // Row 2: Action buttons (bottom row) - Crop, Save, New - 3 buttons
        let row2_y = screen_rect.max.y - BUTTON_RADIUS * 2.0 - SPACING;
        
        // Crop button (left)
        egui::Area::new("crop_btn")
            .fixed_pos(egui::pos2(SPACING, row2_y))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                if self.circular_button_styled(ui, BUTTON_RADIUS, "Crop", "crop", 
                    egui::Color32::from_rgb(60, 60, 70)) {
                    self.current_phase = Phase::Crop;
                    self.crop_rect = None;
                }
            });
        
        // Save button (middle)
        egui::Area::new("save_btn")
            .fixed_pos(egui::pos2(SPACING + BUTTON_RADIUS * 2.0 + SPACING, row2_y))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                if self.circular_button_styled(ui, BUTTON_RADIUS, "Save", "save",
                    egui::Color32::from_rgb(60, 60, 70)) {
                    self.save_and_continue_iteration(ctx);
                }
            });
        
        // New Image button (right)
        egui::Area::new("new_btn")
            .fixed_pos(egui::pos2(SPACING + (BUTTON_RADIUS * 2.0 + SPACING) * 2.0, row2_y))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                if self.circular_button_styled(ui, BUTTON_RADIUS, "New", "new",
                    egui::Color32::from_rgb(60, 60, 70)) {
                    self.start_new_photo_session();
                }
            });
        
        // Optional: Export to USB button if USB present (bottom left corner)
        if self.usb_present() {
            let export_y = screen_rect.max.y - BUTTON_RADIUS - SPACING / 2.0;
            egui::Area::new("export_btn")
                .fixed_pos(egui::pos2(SPACING, export_y))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    if self.circular_button_styled(ui, BUTTON_RADIUS * 0.7, "USB", "export",
                        egui::Color32::from_rgb(40, 80, 40)) {
                        match self.copy_to_usb() {
                            Ok(()) => {
                                self.export_message = Some("✓ Exported to USB!".to_string());
                                self.export_message_time = Some(Instant::now());
                            }
                            Err(e) => {
                                self.export_message = Some(format!("✗ Export failed: {}", e));
                                self.export_message_time = Some(Instant::now());
                            }
                        }
                    }
                });
        }
    }

    // ============================================================================
    // PHASE 3: CROP - Vertical sliders on right, Cancel/Apply on left
    // ============================================================================
    fn render_crop_buttons_circular(&mut self, ctx: &egui::Context, screen_rect: egui::Rect) {
        const BUTTON_RADIUS: f32 = 80.0;  // Larger buttons for better touch targets
        const SPACING: f32 = 20.0;
        
        // Left side: Two buttons stacked vertically
        let left_x = SPACING + BUTTON_RADIUS;
        let button_vertical_spacing = SPACING * 2.0;
        
        // Center buttons vertically
        let total_height = BUTTON_RADIUS * 4.0 + button_vertical_spacing;
        let start_y = (screen_rect.height() - total_height) / 2.0 + screen_rect.min.y;
        
        // Cancel button (top)
        egui::Area::new("cancel_crop_btn")
            .fixed_pos(egui::pos2(left_x, start_y + BUTTON_RADIUS) - egui::vec2(BUTTON_RADIUS, BUTTON_RADIUS))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                if self.circular_button_styled(ui, BUTTON_RADIUS, "Cancel", "cancel",
                    egui::Color32::from_rgb(80, 40, 40)) {
                    self.current_phase = Phase::Edit;
                    self.crop_rect = None;
                }
            });
        
        // Apply Crop button (bottom)
        egui::Area::new("apply_crop_btn")
            .fixed_pos(egui::pos2(
                left_x,
                start_y + BUTTON_RADIUS * 3.0 + button_vertical_spacing
            ) - egui::vec2(BUTTON_RADIUS, BUTTON_RADIUS))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                if self.circular_button_styled(ui, BUTTON_RADIUS, "Apply", "apply",
                    egui::Color32::from_rgb(40, 80, 40)) {
                    self.apply_crop_and_sort(ctx);
                }
            });
    }

    // ============================================================================
    // VERTICAL SLIDERS (for Edit and Crop phases) - Placed horizontally
    // ============================================================================
    fn render_vertical_sliders(&mut self, ctx: &egui::Context, screen_rect: egui::Rect, 
                                slider_width: f32, _slider_height: f32, spacing: f32) {
        // Place sliders side by side on the right edge
        let slider_spacing = spacing;
        
        // More padding at top and bottom to prevent handle cutoff
        let knob_radius = slider_width * 0.6; // Same calculation as in vertical_slider
        let top_padding = spacing * 3.0 + knob_radius; // Extra space for top handle
        let bottom_padding = spacing * 5.0; // Extra space for label and bottom handle
        
        // Stretch sliders to fill screen height (with padding)
        let full_slider_height = screen_rect.height() - top_padding - bottom_padding;
        
        // Start from right edge, moving left
        let slider2_x = screen_rect.max.x - slider_width - spacing;
        let slider1_x = slider2_x - slider_width - slider_spacing;
        
        // Start from top with padding
        let start_y = screen_rect.min.y + top_padding;
        
        // Threshold slider (left one)
        let mut threshold = self.sorting_params.threshold;
        let threshold_changed = egui::Area::new("threshold_slider")
            .fixed_pos(egui::pos2(slider1_x, start_y))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    vertical_slider(ui, &mut threshold, 
                        0.0..=255.0, slider_width, full_slider_height, "Threshold")
                }).inner
            }).inner;
        
        if threshold_changed {
            self.sorting_params.threshold = threshold;
            self.apply_pixel_sort(ctx);
        }
        
        // Hue slider (right one)
        let mut color_tint = self.sorting_params.color_tint;
        let hue_changed = egui::Area::new("hue_slider")
            .fixed_pos(egui::pos2(slider2_x, start_y))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    vertical_slider(ui, &mut color_tint, 
                        0.0..=360.0, slider_width, full_slider_height, "Hue")
                }).inner
            }).inner;
        
        if hue_changed {
            if !self.tint_enabled && color_tint > 0.0 {
                self.tint_enabled = true;
            }
            self.sorting_params.color_tint = color_tint;
            self.apply_pixel_sort(ctx);
        }
    }

    // ============================================================================
    // CIRCULAR BUTTON HELPERS
    // ============================================================================
    
    /// Basic circular button with default styling
    fn circular_button(&self, ui: &mut egui::Ui, radius: f32, text: &str, id: &str) -> bool {
        self.circular_button_styled(ui, radius, text, id, egui::Color32::from_rgba_unmultiplied(70, 70, 80, 180))
    }
    
    /// Circular button with custom fill color
    fn circular_button_styled(&self, ui: &mut egui::Ui, radius: f32, text: &str, 
                               _id: &str, base_fill: egui::Color32) -> bool {
        let size = egui::vec2(radius * 2.0, radius * 2.0);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let center = rect.center();
            
            // Determine colors based on interaction state
            let (fill_color, stroke_color) = if response.is_pointer_button_down_on() {
                // Pressed state - darker
                let r = base_fill.r().saturating_sub(30);
                let g = base_fill.g().saturating_sub(30);
                let b = base_fill.b().saturating_sub(30);
                (egui::Color32::from_rgb(r, g, b), egui::Color32::from_rgb(120, 120, 130))
            } else if response.hovered() {
                // Hovered state - lighter
                let r = base_fill.r().saturating_add(20);
                let g = base_fill.g().saturating_add(20);
                let b = base_fill.b().saturating_add(20);
                (egui::Color32::from_rgb(r, g, b), egui::Color32::from_rgb(150, 150, 160))
            } else {
                // Normal state
                (base_fill, egui::Color32::from_rgb(100, 100, 110))
            };
            
            // Draw shadow for depth
            painter.circle(
                center + egui::vec2(3.0, 3.0),
                radius,
                egui::Color32::from_black_alpha(80),
                egui::Stroke::NONE,
            );
            
            // Draw main circle
            painter.circle(
                center,
                radius,
                fill_color,
                egui::Stroke::new(3.0, stroke_color),
            );
            
            // Draw text in center
            let font_id = egui::FontId::proportional(radius / 3.0); // Scale text with button
            let galley = painter.layout_no_wrap(text.to_string(), font_id, egui::Color32::WHITE);
            let text_pos = center - galley.size() / 2.0;
            painter.galley(text_pos, galley);
            
            // Change cursor on hover
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        
        response.clicked()
    }

    fn cycle_algorithm(&mut self) {
        let all = SortingAlgorithm::all();
        let idx = all.iter().position(|&a| a == self.current_algorithm).unwrap_or(0);
        let next_idx = (idx + 1) % all.len();
        self.current_algorithm = all[next_idx];
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Vertical slider helper function
fn vertical_slider(ui: &mut egui::Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>,
                    width: f32, height: f32, label: &str) -> bool {
    let desired_size = egui::vec2(width, height);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    
    let mut changed = false;
    
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        
        // Background rail
        let rail_rect = rect.shrink2(egui::vec2(width * 0.3, 0.0));
        painter.rect(
            rail_rect,
            rail_rect.width() / 2.0,
            egui::Color32::from_rgb(40, 40, 45),
            egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 90)),
        );
        
        // Calculate normalized position (inverted for vertical)
        let min = *range.start();
        let max = *range.end();
        let normalized = (*value - min) / (max - min);
        
        // Handle dragging
        if response.dragged() || response.clicked() {
            if let Some(mouse_pos) = ui.ctx().pointer_interact_pos() {
                // Invert y-axis (top = max, bottom = min)
                let new_normalized = 1.0 - ((mouse_pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
                *value = min + new_normalized * (max - min);
                changed = true;
                response.mark_changed();
            }
        }
        
        // Filled portion (from bottom up)
        let filled_height = rect.height() * normalized;
        if filled_height > 0.0 {
            let filled_rect = egui::Rect::from_min_max(
                egui::pos2(rail_rect.min.x, rail_rect.max.y - filled_height),
                rail_rect.max,
            );
            painter.rect(
                filled_rect,
                rail_rect.width() / 2.0,
                egui::Color32::from_rgb(80, 120, 200),
                egui::Stroke::NONE,
            );
        }
        
        // Knob/handle
        let knob_y = rect.bottom() - rect.height() * normalized;
        let knob_center = egui::pos2(rect.center().x, knob_y);
        let knob_radius = width * 0.6;
        
        // Draw knob shadow
        painter.circle(
            knob_center + egui::vec2(2.0, 2.0),
            knob_radius,
            egui::Color32::from_black_alpha(60),
            egui::Stroke::NONE,
        );
        
        // Draw knob
        painter.circle(
            knob_center,
            knob_radius,
            egui::Color32::from_rgb(200, 200, 210),
            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 100, 110)),
        );
        
        // Show value bubble when dragging (on top layer to avoid clipping)
        if response.dragged() {
            let text = format!("{:.0}", value);
            let font_id = egui::FontId::proportional(18.0);
            
            // Use a separate layer for the bubble to ensure it's on top
            let layer_id = egui::LayerId::new(egui::Order::Tooltip, ui.id().with("value_bubble"));
            let layer_painter = ui.ctx().layer_painter(layer_id);
            
            let galley = layer_painter.layout_no_wrap(text, font_id, egui::Color32::WHITE);
            
            let bubble_size = galley.size() + egui::vec2(20.0, 12.0);
            let bubble_pos = egui::pos2(rect.left() - bubble_size.x - 12.0, knob_y - bubble_size.y / 2.0);
            let bubble_rect = egui::Rect::from_min_size(bubble_pos, bubble_size);
            
            layer_painter.rect(
                bubble_rect,
                6.0,
                egui::Color32::from_rgb(50, 50, 55),
                egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 120, 130)),
            );
            
            let text_pos = bubble_rect.center() - galley.size() / 2.0;
            layer_painter.galley(text_pos, galley);
        }
        
        // Label below slider (positioned well below to avoid interfering with handle)
        let label_font = egui::FontId::proportional(14.0);
        let label_galley = painter.layout_no_wrap(label.to_string(), label_font, egui::Color32::WHITE);
        let label_pos = egui::pos2(
            rect.center().x - label_galley.size().x / 2.0,
            rect.bottom() + 40.0, // Moved further down to 40.0 to clear bottom handle
        );
        
        // Label background for readability
        let label_bg_rect = egui::Rect::from_min_size(
            label_pos - egui::vec2(4.0, 2.0),
            label_galley.size() + egui::vec2(8.0, 4.0),
        );
        painter.rect(
            label_bg_rect,
            3.0,
            egui::Color32::from_black_alpha(180),
            egui::Stroke::NONE,
        );
        painter.galley(label_pos, label_galley);
    }
    
    changed
}

// Helper functions for image centering
fn fit_image_in_rect(image_size: egui::Vec2, container_size: egui::Vec2) -> egui::Vec2 {
    let scale = (container_size.x / image_size.x).min(container_size.y / image_size.y);
    image_size * scale
}

fn center_rect_in_rect(content_size: egui::Vec2, container: egui::Rect) -> egui::Rect {
    let offset = (container.size() - content_size) * 0.5;
    egui::Rect::from_min_size(container.min + offset, content_size)
}

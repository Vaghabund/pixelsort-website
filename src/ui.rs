use std::sync::Arc;
use std::time::Instant;
use eframe::egui;
use tokio::sync::RwLock;

use crate::pixel_sorter::{PixelSorter, SortingAlgorithm, SortingParameters};
use crate::camera_controller::CameraController;

// ============================================================================
// CONSTANTS FOR UI STYLING - Easy to modify
// ============================================================================
const BUTTON_HEIGHT: f32 = 50.0;
const BUTTON_SPACING: f32 = 10.0;
const HANDLE_SIZE: f32 = 20.0;

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
            tint_enabled: false,
        }
    }

    fn usb_present(&self) -> bool {
        let usb_paths = ["/media/pi", "/media", "/mnt"];
        for base_path in &usb_paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
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
        // Update camera preview at 30 FPS if in Input phase
        if self.current_phase == Phase::Input && !self.is_processing {
            self.update_camera_preview(ctx);
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

    fn render_ui(&mut self, ctx: &egui::Context) {
        // Calculate dynamic button zone height based on current phase
        let button_zone_height = match self.current_phase {
            Phase::Input => BUTTON_HEIGHT + BUTTON_SPACING * 2.0, // 1 row
            Phase::Crop => BUTTON_HEIGHT + BUTTON_SPACING * 2.0, // 1 row
            Phase::Edit => {
                // 2 rows (sliders + buttons) + optional USB row
                let base_height = (BUTTON_HEIGHT + 60.0) + BUTTON_SPACING * 3.0;
                if self.usb_present() {
                    base_height + BUTTON_HEIGHT + BUTTON_SPACING
                } else {
                    base_height
                }
            }
        };
        
        // Button Zone at bottom
        egui::TopBottomPanel::bottom("button_zone")
            .exact_height(button_zone_height)
            .show(ctx, |ui| {
                self.render_button_zone(ui, ctx);
            });

        // Viewport in center (full screen now without status bar)
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_viewport(ui, ctx);
        });
    }
}

// ============================================================================
// VIEWPORT RENDERING
// ============================================================================

impl PixelSorterApp {
    fn render_viewport(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let viewport_rect = ui.max_rect();

        match self.current_phase {
            Phase::Input => self.render_input_viewport(ui, viewport_rect),
            Phase::Edit => self.render_edit_viewport(ui, viewport_rect),
            Phase::Crop => self.render_crop_viewport(ui, viewport_rect, ctx),
        }
    }

    fn render_input_viewport(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        if let Some(texture) = &self.camera_texture {
            let image_size = texture.size_vec2();
            let display_size = fit_rect_in_rect(image_size, rect.size());
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
        if let Some(texture) = &self.processed_texture {
            let image_size = texture.size_vec2();
            let display_size = fit_rect_in_rect(image_size, rect.size());
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
        if let Some(texture) = &self.processed_texture {
            let image_size = texture.size_vec2();
            let display_size = fit_rect_in_rect(image_size, rect.size());
            let centered_rect = center_rect_in_rect(display_size, rect);
            
            // Draw image
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
// BUTTON ZONE RENDERING
// ============================================================================

impl PixelSorterApp {
    fn render_button_zone(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Use ScrollArea to ensure all buttons are accessible even on smaller screens
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(BUTTON_SPACING);
                    
                    match self.current_phase {
                        Phase::Input => self.render_input_buttons(ui, ctx),
                        Phase::Edit => self.render_edit_buttons(ui, ctx),
                        Phase::Crop => self.render_crop_buttons(ui, ctx),
                    }
                    
                    ui.add_space(BUTTON_SPACING);
                });
            });
    }

    fn render_input_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0; // Remove default spacing
            let total_width = ui.available_width();
            let button_width = (total_width - BUTTON_SPACING) / 2.0;
            
            if ui.add_sized(egui::vec2(button_width, BUTTON_HEIGHT), egui::Button::new("Take Picture")).clicked() {
                self.capture_and_sort(ctx);
            }
            
            ui.add_space(BUTTON_SPACING);
            
            if ui.add_sized(egui::vec2(button_width, BUTTON_HEIGHT), egui::Button::new("Upload Image")).clicked() {
                self.load_image(ctx);
            }
        });
    }

    fn render_edit_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Row 1: Two sliders side by side - Threshold and Tint (Hue)
        ui.horizontal(|ui| {
            let available_width = ui.available_width();
            let slider_width = (available_width - BUTTON_SPACING * 3.0) / 2.0; // Split width equally with spacing
            
            // Left side: Threshold slider
            ui.allocate_ui(egui::vec2(slider_width, 60.0), |ui| {
                ui.vertical(|ui| {
                    if touch_slider(ui, &mut self.sorting_params.threshold, 0.0..=255.0).changed() {
                        self.apply_pixel_sort(ctx);
                    }
                    ui.centered_and_justified(|ui| {
                        ui.label("Threshold");
                    });
                });
            });
            
            ui.add_space(BUTTON_SPACING * 3.0);
            
            // Right side: Tint (Hue) slider
            ui.allocate_ui(egui::vec2(slider_width, 60.0), |ui| {
                ui.vertical(|ui| {
                    let slider_response = touch_slider(ui, &mut self.sorting_params.color_tint, 0.0..=360.0);
                    if slider_response.changed() {
                        // Auto-enable tint when user adjusts the slider
                        if !self.tint_enabled && self.sorting_params.color_tint > 0.0 {
                            self.tint_enabled = true;
                        }
                        self.apply_pixel_sort(ctx);
                    }
                    
                    ui.centered_and_justified(|ui| {
                        ui.label("Hue");
                    });
                });
            });
        });

        ui.add_space(BUTTON_SPACING);

        // Row 2: All buttons in one row
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0; // Remove default spacing
            let total_width = ui.available_width();
            let spacing_width = BUTTON_SPACING * 4.0; // Manual spacing between 5 buttons
            let button_width = (total_width - spacing_width) / 5.0;
            
            // Mode buttons (lighter - default styling)
            if ui.add_sized(egui::vec2(button_width, BUTTON_HEIGHT), egui::Button::new(self.current_algorithm.name())).clicked() {
                self.cycle_algorithm();
                self.apply_pixel_sort(ctx);
            }
            
            ui.add_space(BUTTON_SPACING);
            
            if ui.add_sized(egui::vec2(button_width, BUTTON_HEIGHT), egui::Button::new(self.sorting_params.sort_mode.name())).clicked() {
                self.sorting_params.sort_mode = self.sorting_params.sort_mode.next();
                self.apply_pixel_sort(ctx);
            }
            
            ui.add_space(BUTTON_SPACING);
            
            // Action buttons (darker)
            let action_button = egui::Button::new("Crop").fill(egui::Color32::from_rgb(60, 60, 70));
            if ui.add_sized(egui::vec2(button_width, BUTTON_HEIGHT), action_button).clicked() {
                self.current_phase = Phase::Crop;
                self.crop_rect = None; // Reset crop
            }
            
            ui.add_space(BUTTON_SPACING);
            
            let action_button = egui::Button::new("Save & Iterate").fill(egui::Color32::from_rgb(60, 60, 70));
            if ui.add_sized(egui::vec2(button_width, BUTTON_HEIGHT), action_button).clicked() {
                self.save_and_continue_iteration(ctx);
            }
            
            ui.add_space(BUTTON_SPACING);
            
            let action_button = egui::Button::new("New Image").fill(egui::Color32::from_rgb(60, 60, 70));
            if ui.add_sized(egui::vec2(button_width, BUTTON_HEIGHT), action_button).clicked() {
                self.start_new_photo_session();
            }
        });

        ui.add_space(BUTTON_SPACING);

        // Row 3: Export button (if USB present)
        if self.usb_present() {
            ui.horizontal(|ui| {
                if equal_button(ui, "Export to USB", BUTTON_HEIGHT, 1).clicked() {
                    let _ = self.copy_to_usb(); // Silently attempt export
                }
            });
        }
    }

    fn render_crop_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Simple crop controls: Cancel and Apply
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0; // Remove default spacing
            let total_width = ui.available_width();
            let button_width = (total_width - BUTTON_SPACING) / 2.0;
            
            let action_button = egui::Button::new("Cancel").fill(egui::Color32::from_rgb(60, 60, 70));
            if ui.add_sized(egui::vec2(button_width, BUTTON_HEIGHT), action_button).clicked() {
                self.current_phase = Phase::Edit;
                self.crop_rect = None;
            }
            
            ui.add_space(BUTTON_SPACING);
            
            let action_button = egui::Button::new("Apply Crop").fill(egui::Color32::from_rgb(60, 60, 70));
            if ui.add_sized(egui::vec2(button_width, BUTTON_HEIGHT), action_button).clicked() {
                self.apply_crop_and_sort(ctx);
            }
        });
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

/// Custom slider that shows value in a bubble only when touched/dragged
fn touch_slider(ui: &mut egui::Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>) -> egui::Response {
    let desired_size = egui::vec2(ui.available_width() - 20.0, 20.0); // Added side padding
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    
    // Add horizontal padding
    let rect = rect.shrink2(egui::vec2(10.0, 0.0));
    
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        
        // Background rail
        let rail_rect = rect.shrink2(egui::vec2(0.0, rect.height() * 0.25));
        ui.painter().rect(
            rail_rect,
            rail_rect.height() / 2.0,
            ui.visuals().widgets.inactive.bg_fill,
            visuals.bg_stroke,
        );
        
        // Calculate the position
        let min = *range.start();
        let max = *range.end();
        let normalized = (*value - min) / (max - min);
        
        // Handle dragging
        if response.dragged() {
            if let Some(mouse_pos) = ui.ctx().pointer_interact_pos() {
                let new_normalized = ((mouse_pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                *value = min + new_normalized * (max - min);
                response.mark_changed();
            }
        }
        
        // Handle direct click
        if response.clicked() {
            if let Some(mouse_pos) = ui.ctx().pointer_interact_pos() {
                let new_normalized = ((mouse_pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                *value = min + new_normalized * (max - min);
                response.mark_changed();
            }
        }
        
        // Filled portion
        let filled_width = rect.width() * normalized;
        if filled_width > 0.0 {
            let filled_rect = egui::Rect::from_min_max(
                rail_rect.min,
                egui::pos2(rail_rect.min.x + filled_width, rail_rect.max.y),
            );
            ui.painter().rect(
                filled_rect,
                rail_rect.height() / 2.0,
                ui.visuals().selection.bg_fill,
                egui::Stroke::NONE,
            );
        }
        
        // Knob/handle
        let knob_pos = rect.left() + rect.width() * normalized;
        let knob_center = egui::pos2(knob_pos, rect.center().y);
        let knob_radius = rect.height() * 0.8;
        
        ui.painter().circle(
            knob_center,
            knob_radius,
            visuals.bg_fill,
            visuals.fg_stroke,
        );
        
        // Show value bubble ONLY when actively dragging (not just hovering)
        // Use a layer to ensure it's always on top
        if response.dragged() {
            let text = format!("{:.0}", value);
            let font_id = egui::FontId::proportional(16.0);
            let galley = ui.painter().layout_no_wrap(text, font_id.clone(), visuals.text_color());
            
            // Bubble background
            let bubble_size = galley.size() + egui::vec2(16.0, 10.0);
            let bubble_pos = egui::pos2(knob_pos - bubble_size.x / 2.0, rect.top() - bubble_size.y - 12.0);
            let bubble_rect = egui::Rect::from_min_size(bubble_pos, bubble_size);
            
            // Draw on a higher layer to ensure visibility
            let layer_id = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("slider_bubble"));
            ui.ctx().layer_painter(layer_id).rect(
                bubble_rect,
                5.0,
                egui::Color32::from_rgb(40, 40, 45),
                egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 110)),
            );
            
            // Text
            let text_pos = bubble_rect.center() - galley.size() / 2.0;
            ui.ctx().layer_painter(layer_id).galley(text_pos, galley);
        }
    }
    
    response
}

fn equal_button(ui: &mut egui::Ui, text: &str, height: f32, count: usize) -> egui::Response {
    let spacing_total = BUTTON_SPACING * (count - 1) as f32;
    let width = (ui.available_width() - spacing_total) / count as f32;
    ui.add_sized(
        egui::vec2(width, height),
        egui::Button::new(text),
    )
}

fn fit_rect_in_rect(content: egui::Vec2, container: egui::Vec2) -> egui::Vec2 {
    let scale = (container.x / content.x).min(container.y / content.y);
    content * scale
}

fn center_rect_in_rect(content_size: egui::Vec2, container: egui::Rect) -> egui::Rect {
    let offset = (container.size() - content_size) * 0.5;
    egui::Rect::from_min_size(container.min + offset, content_size)
}
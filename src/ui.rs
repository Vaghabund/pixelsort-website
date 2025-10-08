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
use crate::image_processor::ImageProcessor;
use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CropAspectRatio {
    Square,     // 1:1
    Portrait,   // 3:4
    Landscape,  // 16:9
}

impl CropAspectRatio {
    pub fn all() -> &'static [CropAspectRatio] {
        &[
            CropAspectRatio::Square,
            CropAspectRatio::Portrait,
            CropAspectRatio::Landscape,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            CropAspectRatio::Square => "1:1",
            CropAspectRatio::Portrait => "3:4",
            CropAspectRatio::Landscape => "16:9",
        }
    }

    pub fn ratio(&self) -> f32 {
        match self {
            CropAspectRatio::Square => 1.0,
            CropAspectRatio::Portrait => 3.0 / 4.0,
            CropAspectRatio::Landscape => 16.0 / 9.0,
        }
    }
}

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
    pub crop_rect: Option<egui::Rect>,
    pub selection_start: Option<egui::Pos2>,
    pub crop_aspect_ratio: CropAspectRatio,
    pub crop_rotation: i32, // degrees (0,90,180,270)
    pub was_cropped: bool,
    pub crop_dragging: Option<CropDragAction>,
    pub drag_start_pos: Option<egui::Pos2>,
    pub drag_start_rect: Option<egui::Rect>,
}

impl PixelSorterApp {
    pub fn new(
        pixel_sorter: Arc<PixelSorter>,
        image_processor: Arc<RwLock<ImageProcessor>>,
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
            image_processor,
            config,
            preview_mode: true,
            iteration_counter: 0,
            current_session_folder: None,
            last_camera_update: None,
            // Crop functionality
            crop_mode: false,
            crop_rect: None,
            selection_start: None,
            crop_aspect_ratio: CropAspectRatio::Square,
            crop_rotation: 0,
            was_cropped: false,
            crop_dragging: None,
            drag_start_pos: None,
            drag_start_rect: None,
        }
    }

    // Camera preview is now handled directly in the main update loop with throttling
}

impl eframe::App for PixelSorterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        // Layout: left controls side panel + right image area
        egui::SidePanel::left("left_controls").resizable(true).show(ctx, |ui| {
            ui.visuals_mut().window_fill = egui::Color32::from_black_alpha(180);
            egui::Frame::window(&ui.style()).show(ui, |ui| {
                ui.heading("Pixel Sorter");
                ui.add_space(6.0);

                // Reuse the previous control layout (algorithm, sliders, crop controls, action buttons)
                // ...existing control UI...
                // We'll insert the controls by calling a helper closure below
                
                // Algorithm and parameters
                ui.vertical(|ui| {
                    // Algorithm and parameters
                    ui.horizontal(|ui| {
                        ui.label("Algorithm:");
                        egui::ComboBox::from_id_source("sorting_algorithm")
                            .selected_text(self.current_algorithm.name())
                            .show_ui(ui, |ui| {
                                for &algorithm in SortingAlgorithm::all() {
                                    if ui.selectable_value(&mut self.current_algorithm, algorithm, algorithm.name()).clicked() {
                                        self.apply_pixel_sort(ctx);
                                    }
                                }
                            });

                        ui.add_space(10.0);

                        ui.label(format!("Color Tint: {:.0}°", self.sorting_params.color_tint));
                        let tint_changed = ui.add(
                            egui::Slider::new(&mut self.sorting_params.color_tint, 0.0..=360.0)
                                .step_by(1.0)
                                .show_value(false)
                        ).changed();

                        ui.add_space(10.0);

                        ui.label(format!("Threshold: {:.0}", self.sorting_params.threshold));
                        let threshold_changed = ui.add(
                            egui::Slider::new(&mut self.sorting_params.threshold, 0.0..=255.0)
                                .step_by(1.0)
                                .show_value(false)
                        ).changed();

                        if (tint_changed || threshold_changed) && !self.is_processing {
                            self.apply_pixel_sort(ctx);
                        }
                    });

                    ui.add_space(6.0);

                    // Crop controls
                    ui.horizontal(|ui| {
                        if ui.button(if self.crop_mode { "Cancel Crop" } else { "Select Crop" }).clicked() {
                            self.crop_mode = !self.crop_mode;
                            if !self.crop_mode {
                                self.crop_rect = None;
                                self.selection_start = None;
                            }
                        }

                        if self.crop_mode {
                            ui.separator();
                            ui.label("Aspect Ratio:");
                            egui::ComboBox::from_id_source("crop_aspect_ratio")
                                .selected_text(self.crop_aspect_ratio.name())
                                .show_ui(ui, |ui| {
                                    for &ratio in CropAspectRatio::all() {
                                        ui.selectable_value(&mut self.crop_aspect_ratio, ratio, ratio.name());
                                    }
                                });

                            ui.separator();
                            if ui.button("Rotate 90°").clicked() {
                                self.crop_rotation = (self.crop_rotation + 90) % 360;
                                // Create an immediate preview of the rotated crop (no sorting) so user sees rotation
                                if let (Some(ref original), Some(crop_rect)) = (&self.original_image, self.crop_rect) {
                                    let screen_rect = ctx.screen_rect();
                                    let image_size = original.dimensions();
                                    let scale_x = image_size.0 as f32 / screen_rect.width();
                                    let scale_y = image_size.1 as f32 / screen_rect.height();

                                    let crop_min_x = (crop_rect.min.x * scale_x).max(0.0).min(image_size.0 as f32) as u32;
                                    let crop_min_y = (crop_rect.min.y * scale_y).max(0.0).min(image_size.1 as f32) as u32;
                                    let crop_max_x = (crop_rect.max.x * scale_x).max(0.0).min(image_size.0 as f32) as u32;
                                    let crop_max_y = (crop_rect.max.y * scale_y).max(0.0).min(image_size.1 as f32) as u32;

                                    let crop_width = crop_max_x.saturating_sub(crop_min_x);
                                    let crop_height = crop_max_y.saturating_sub(crop_min_y);

                                    if crop_width > 0 && crop_height > 0 {
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

                                        let rotated = match self.crop_rotation {
                                            90 => image::imageops::rotate90(&cropped),
                                            180 => image::imageops::rotate180(&cropped),
                                            270 => image::imageops::rotate270(&cropped),
                                            _ => cropped,
                                        };

                                        // Show rotated preview (no sorting) - immediate visual feedback
                                        self.processed_image = Some(rotated.clone());
                                        self.create_processed_texture(ctx, rotated);
                                        self.was_cropped = true;
                                    }
                                }
                            }
                            ui.label(format!("{}°", self.crop_rotation));
                        }

                        if self.crop_mode && self.crop_rect.is_some() {
                            ui.separator();
                            if ui.button("Apply Crop").clicked() {
                                self.apply_crop_and_sort(ctx);
                            }
                        }
                    });

                    ui.add_space(8.0);

                    // Action buttons
                    ui.horizontal(|ui| {
                        if ui.button("Process Image").clicked() && !self.is_processing {
                            self.process_image(ctx);
                        }

                        ui.separator();

                        if ui.button("Save & Continue").clicked() {
                            self.save_and_continue_iteration(ctx);
                        }

                        ui.separator();

                        if ui.button("Back to Camera").clicked() {
                            self.start_new_photo_session();
                        }

                        ui.separator();

                        if ui.button("Export to USB").clicked() {
                            match self.copy_to_usb() {
                                Ok(()) => self.status_message = "Successfully copied to USB!".to_string(),
                                Err(e) => self.status_message = format!("USB copy failed: {}", e),
                            }
                        }
                    });
                });
            });
        });

        // Right-side area: image display fills remaining space
        egui::CentralPanel::default().show(ctx, |ui| {
            let screen_rect = ui.max_rect();
            // Fill with image or prompt
            if self.preview_mode {
                // Show camera preview or prompt
                if let Some(ref _camera) = self.camera_controller {
                    if let Some(texture) = &self.camera_texture {
                        ui.allocate_ui_at_rect(screen_rect, |ui| {
                            ui.add_sized(screen_rect.size(), egui::Image::new(texture));
                        });
                    } else {
                        ui.centered_and_justified(|ui| { ui.label("Initializing camera..."); });
                    }
                } else {
                    ui.centered_and_justified(|ui| { ui.label("No camera available - Load an image to begin"); });
                }
            } else {
                // Show processed image with zoom and crop support
                if let Some(texture) = self.processed_texture.clone() {
                    ui.allocate_ui_at_rect(screen_rect, |ui| {
                        // Handle mouse interactions for crop selection
                        let response = ui.interact(screen_rect, egui::Id::new("image_interaction"), egui::Sense::click_and_drag());
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
                if let Some(texture) = self.processed_texture.clone() {
                    ui.allocate_ui_at_rect(screen_rect, |ui| {
                        // Handle mouse interactions for crop selection
                        let response = ui.interact(screen_rect, egui::Id::new("image_interaction"), egui::Sense::click_and_drag());

                        if self.crop_mode {
                            // Corner/center drag interaction for a movable/resizable crop rectangle
                            let pointer = response.interact_pointer_pos();

                            // Helper to find if pointer near a corner
                            let corner_hit = |rect: egui::Rect, p: egui::Pos2, tol: f32| -> Option<CropDragAction> {
                                if (p - rect.min).length() <= tol { return Some(CropDragAction::ResizeTopLeft); }
                                if (p - egui::pos2(rect.max.x, rect.min.y)).length() <= tol { return Some(CropDragAction::ResizeTopRight); }
                                if (p - egui::pos2(rect.min.x, rect.max.y)).length() <= tol { return Some(CropDragAction::ResizeBottomLeft); }
                                if (p - rect.max).length() <= tol { return Some(CropDragAction::ResizeBottomRight); }
                                None
                            };

                            if response.drag_started() {
                                // Start either a new crop or begin interaction with existing crop
                                let start = pointer.unwrap_or_default();
                                self.selection_start = Some(start);

                                if let Some(rect) = self.crop_rect {
                                    // Check if user started near a corner or center for move
                                    if let Some(action) = corner_hit(rect, start, 16.0) {
                                        self.crop_dragging = Some(action);
                                        self.drag_start_pos = Some(start);
                                        self.drag_start_rect = Some(rect);
                                    } else if rect.contains(start) {
                                        self.crop_dragging = Some(CropDragAction::Move);
                                        self.drag_start_pos = Some(start);
                                        self.drag_start_rect = Some(rect);
                                    } else {
                                        // started outside existing rect -> start new selection
                                        self.crop_rect = None;
                                        self.crop_dragging = None;
                                    }
                                } else {
                                    // starting a new selection
                                    self.crop_rect = None;
                                    self.crop_dragging = None;
                                }
                            } else if response.dragged() {
                                if let Some(start) = self.selection_start {
                                    if let Some(current) = pointer {
                                        // If we have a drag action, perform it
                                        if let Some(action) = self.crop_dragging {
                                            if let (Some(spos), Some(srect)) = (self.drag_start_pos, self.drag_start_rect) {
                                                let dx = current.x - spos.x;
                                                let dy = current.y - spos.y;

                                                match action {
                                                    CropDragAction::Move => {
                                                        let mut new_min = egui::pos2(srect.min.x + dx, srect.min.y + dy);
                                                        let mut new_max = egui::pos2(srect.max.x + dx, srect.max.y + dy);
                                                        // keep within screen
                                                        let w = new_max.x - new_min.x;
                                                        let h = new_max.y - new_min.y;
                                                        if new_min.x < screen_rect.min.x { new_min.x = screen_rect.min.x; new_max.x = new_min.x + w; }
                                                        if new_max.x > screen_rect.max.x { new_max.x = screen_rect.max.x; new_min.x = new_max.x - w; }
                                                        if new_min.y < screen_rect.min.y { new_min.y = screen_rect.min.y; new_max.y = new_min.y + h; }
                                                        if new_max.y > screen_rect.max.y { new_max.y = screen_rect.max.y; new_min.y = new_max.y - h; }
                                                        self.crop_rect = Some(egui::Rect::from_min_max(new_min, new_max));
                                                    }
                                                    _ => {
                                                        // Resize from corner while preserving aspect ratio
                                                        let target_ratio = self.crop_aspect_ratio.ratio();
                                                        // start rect pixel coords
                                                        let smin = srect.min;
                                                        let smax = srect.max;
                                                        let mut min = smin;
                                                        let mut max = smax;
                                                        match action {
                                                            CropDragAction::ResizeTopLeft => {
                                                                min = egui::pos2(smin.x + dx, smin.y + dy);
                                                            }
                                                            CropDragAction::ResizeTopRight => {
                                                                max = egui::pos2(smax.x + dx, smin.y + dy);
                                                            }
                                                            CropDragAction::ResizeBottomLeft => {
                                                                min = egui::pos2(smin.x + dx, smax.y + dy);
                                                            }
                                                            CropDragAction::ResizeBottomRight => {
                                                                max = egui::pos2(smax.x + dx, smax.y + dy);
                                                            }
                                                            _ => {}
                                                        }

                                                        // Compute width/height keeping aspect ratio centered on the opposite corner
                                                        let mut width = (max.x - min.x).abs();
                                                        let mut height = (max.y - min.y).abs();
                                                        if width / height > target_ratio {
                                                            width = height * target_ratio;
                                                        } else {
                                                            height = width / target_ratio;
                                                        }

                                                        // Reconstruct rect based on which corner was moved
                                                        match action {
                                                            CropDragAction::ResizeTopLeft => {
                                                                max = srect.max;
                                                                min = egui::pos2(max.x - width, max.y - height);
                                                            }
                                                            CropDragAction::ResizeTopRight => {
                                                                min = srect.min;
                                                                max = egui::pos2(min.x + width, min.y + height);
                                                            }
                                                            CropDragAction::ResizeBottomLeft => {
                                                                min = egui::pos2(srect.min.x, srect.min.y);
                                                                max = egui::pos2(min.x + width, min.y + height);
                                                            }
                                                            CropDragAction::ResizeBottomRight => {
                                                                min = srect.min;
                                                                max = egui::pos2(min.x + width, min.y + height);
                                                            }
                                                            _ => {}
                                                        }

                                                        // clamp to screen
                                                        if min.x < screen_rect.min.x { let shift = screen_rect.min.x - min.x; min.x += shift; max.x += shift; }
                                                        if max.x > screen_rect.max.x { let shift = max.x - screen_rect.max.x; min.x -= shift; max.x -= shift; }
                                                        if min.y < screen_rect.min.y { let shift = screen_rect.min.y - min.y; min.y += shift; max.y += shift; }
                                                        if max.y > screen_rect.max.y { let shift = max.y - screen_rect.max.y; min.y -= shift; max.y -= shift; }

                                                        self.crop_rect = Some(egui::Rect::from_min_max(min, max));
                                                    }
                                                }
                                            }
                                        } else {
                                            // No drag action yet: start a new selection rectangle
                                            let rect = self.constrain_crop_rect(start, current, screen_rect);
                                            self.crop_rect = Some(rect);
                                        }
                                    }
                                }
                            } else if response.drag_released() {
                                // finalize drag
                                self.crop_dragging = None;
                                self.drag_start_pos = None;
                                self.drag_start_rect = None;
                                self.selection_start = None;
                            }
                        } else {
                            // No panning needed without zoom
                        }

                        // Display the image while preserving aspect ratio. Compute the largest
                        // scale that fits the image inside the screen rect (no stretching),
                        // center it and draw. This zooms small cropped images as much as
                        // possible without changing aspect ratio.
                        if let Some(ref img) = self.processed_image {
                            let img_w = img.width() as f32;
                            let img_h = img.height() as f32;
                            let screen_w = screen_rect.width();
                            let screen_h = screen_rect.height();

                            // scale to fit (contain) - largest uniform scale where both dims <= screen
                            let scale = (screen_w / img_w).min(screen_h / img_h);

                            let dest_w = img_w * scale;
                            let dest_h = img_h * scale;

                            let center = screen_rect.center();
                            let dest_min = egui::pos2(center.x - dest_w / 2.0, center.y - dest_h / 2.0);
                            let dest_max = egui::pos2(center.x + dest_w / 2.0, center.y + dest_h / 2.0);
                            let dest_rect = egui::Rect::from_min_max(dest_min, dest_max);

                            ui.painter().image(
                                texture.id(),
                                dest_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                egui::Color32::WHITE,
                            );
                        } else {
                            // Fallback: stretch to full screen if we don't have image dimensions
                            ui.painter().image(
                                texture.id(),
                                screen_rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                egui::Color32::WHITE,
                            );
                        }

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
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            });
                        });
                    } else {
                        // Image Editing Layout
                        ui.vertical(|ui| {
                            // Algorithm and parameters
                            ui.horizontal(|ui| {
                                ui.label("Algorithm:");
                                egui::ComboBox::from_id_source("sorting_algorithm")
                                    .selected_text(self.current_algorithm.name())
                                    .show_ui(ui, |ui| {
                                        for &algorithm in SortingAlgorithm::all() {
                                            if ui.selectable_value(&mut self.current_algorithm, algorithm, algorithm.name()).clicked() {
                                                self.apply_pixel_sort(ctx);
                                            }
                                        }
                                    });

                                ui.add_space(15.0);

                                // Color Tint Slider
                                ui.label(format!("Color Tint: {:.0}°", self.sorting_params.color_tint));
                                let tint_changed = ui.add(
                                    egui::Slider::new(&mut self.sorting_params.color_tint, 0.0..=360.0)
                                        .step_by(1.0)
                                        .show_value(false)
                                ).changed();

                                ui.add_space(10.0);

                                // Threshold Slider
                                ui.label(format!("Threshold: {:.0}", self.sorting_params.threshold));
                                let threshold_changed = ui.add(
                                    egui::Slider::new(&mut self.sorting_params.threshold, 0.0..=255.0)
                                        .step_by(1.0)
                                        .show_value(false)
                                ).changed();

                                if (tint_changed || threshold_changed) && !self.is_processing {
                                    self.apply_pixel_sort(ctx);
                                }
                            });

                            ui.add_space(10.0);

                            // Crop controls
                            ui.horizontal(|ui| {
                                if ui.button(if self.crop_mode { "Cancel Crop" } else { "Select Crop" }).clicked() {
                                    self.crop_mode = !self.crop_mode;
                                    if !self.crop_mode {
                                        self.crop_rect = None;
                                        self.selection_start = None;
                                    }
                                }

                                if self.crop_mode {
                                    ui.separator();
                                    
                                    // Aspect ratio selection (only visible in crop mode)
                                    ui.label("Aspect Ratio:");
                                    egui::ComboBox::from_id_source("crop_aspect_ratio")
                                        .selected_text(self.crop_aspect_ratio.name())
                                        .show_ui(ui, |ui| {
                                            for &ratio in CropAspectRatio::all() {
                                                ui.selectable_value(&mut self.crop_aspect_ratio, ratio, ratio.name());
                                            }
                                        });
                                    ui.separator();

                                    // Rotation and preview controls for crop
                                    ui.horizontal(|ui| {
                                        if ui.button("Rotate 90°").clicked() {
                                            self.crop_rotation = (self.crop_rotation + 90) % 360;
                                        }

                                        if ui.button("Preview Crop").clicked() {
                                            // Create a temporary preview by applying the crop without committing
                                            // We'll create a processed texture from the cropped region and mark was_cropped true
                                            if let (Some(ref original), Some(crop_rect)) = (&self.original_image, self.crop_rect) {
                                                // Reuse apply_crop_and_sort logic but avoid replacing original_image permanently
                                                // We'll perform the crop and create a processed texture
                                                let screen_rect = ctx.screen_rect();
                                                let image_size = original.dimensions();
                                                let scale_x = image_size.0 as f32 / screen_rect.width();
                                                let scale_y = image_size.1 as f32 / screen_rect.height();

                                                let crop_min_x = (crop_rect.min.x * scale_x).max(0.0).min(image_size.0 as f32) as u32;
                                                let crop_min_y = (crop_rect.min.y * scale_y).max(0.0).min(image_size.1 as f32) as u32;
                                                let crop_max_x = (crop_rect.max.x * scale_x).max(0.0).min(image_size.0 as f32) as u32;
                                                let crop_max_y = (crop_rect.max.y * scale_y).max(0.0).min(image_size.1 as f32) as u32;

                                                let crop_width = crop_max_x.saturating_sub(crop_min_x);
                                                let crop_height = crop_max_y.saturating_sub(crop_min_y);

                                                if crop_width > 0 && crop_height > 0 {
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

                                                    // Apply rotation if needed
                                                    let rotated = match self.crop_rotation {
                                                        90 => image::imageops::rotate90(&cropped),
                                                        180 => image::imageops::rotate180(&cropped),
                                                        270 => image::imageops::rotate270(&cropped),
                                                        _ => cropped,
                                                    };

                                                    // Apply pixel sorting to the cropped preview
                                                    let algorithm = self.current_algorithm;
                                                    let params = self.sorting_params.clone();
                                                    let pixel_sorter = Arc::clone(&self.pixel_sorter);
                                                    if let Ok(sorted_cropped) = pixel_sorter.sort_pixels(&rotated, algorithm, &params) {
                                                        // Create a processed texture from the sorted crop and mark was_cropped true
                                                        self.processed_image = Some(sorted_cropped.clone());
                                                        self.create_processed_texture(ctx, sorted_cropped);
                                                        self.was_cropped = true;
                                                    }
                                                }
                                            }
                                        }
                                    });
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
                                // Process Image button
                                if ui.button("Process Image").clicked() && !self.is_processing {
                                    self.process_image(ctx);
                                }

                                ui.separator();

                                // Save & Continue button
                                if ui.button("Save & Continue").clicked() {
                                    self.save_and_continue_iteration(ctx);
                                }

                                ui.separator();

                                // Save As button - user can choose a location to save the processed image
                                if ui.button("Save As...").clicked() {
                                    self.save_image();
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



        // High-performance 30 FPS repaints for smooth camera feed
        if self.preview_mode && self.camera_controller.is_some() && !self.is_processing {
            ctx.request_repaint_after(std::time::Duration::from_millis(33)); // 30 FPS
        }
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

            // Synchronous processing - consider making async in future
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
            self.is_processing = true;
            self.status_message = "Loading image...".to_string();

            match image::open(&path) {
                Ok(img) => {
                    let rgb_image = img.to_rgb8();
                    self.original_image = Some(rgb_image);
                    self.was_cropped = false; // new image resets crop state
                    self.processed_image = None; // Clear any previous processed image
                    self.preview_mode = false; // Switch to editing mode when loading image

                    // Create texture for the loaded image
                        if let Some(ref original) = self.original_image {
                            self.create_processed_texture(ctx, original.clone());
                        }

                    self.is_processing = false;
                    self.status_message = format!("Loaded: {}", path.display());
                }
                Err(e) => {
                    self.is_processing = false;
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
                texture.set(color_image, egui::TextureOptions::NEAREST);
            }
            None => {
                // First time only
                let texture = ctx.load_texture("camera_preview", color_image, egui::TextureOptions::NEAREST);
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
                texture.set(color_image, egui::TextureOptions::NEAREST);
            }
            None => {
                let texture = ctx.load_texture("processed_image", color_image, egui::TextureOptions::NEAREST);
                self.processed_texture = Some(texture);
            }
        }
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

    // reset crop-specific state
    self.crop_rotation = 0;
    self.was_cropped = false;

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

            // Calculate scaling factors (image fills screen)
            let scale_x = image_size.0 as f32 / screen_rect.width();
            let scale_y = image_size.1 as f32 / screen_rect.height();

            // Convert crop rectangle screen coordinates to image coordinates
            let crop_min_x = (crop_rect.min.x * scale_x).max(0.0).min(image_size.0 as f32) as u32;
            let crop_min_y = (crop_rect.min.y * scale_y).max(0.0).min(image_size.1 as f32) as u32;
            let crop_max_x = (crop_rect.max.x * scale_x).max(0.0).min(image_size.0 as f32) as u32;
            let crop_max_y = (crop_rect.max.y * scale_y).max(0.0).min(image_size.1 as f32) as u32;

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
                        // Use nearest filtering for cropped images so the upscaled look is crisp
                        self.create_processed_texture(ctx, sorted_cropped);

                        // Exit crop mode
                        self.crop_mode = false;
                        self.crop_rect = None;
                        self.selection_start = None;

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

    fn constrain_crop_rect(&self, start: egui::Pos2, current: egui::Pos2, screen_rect: egui::Rect) -> egui::Rect {
        let target_ratio = self.crop_aspect_ratio.ratio();
        
        // Calculate the base rectangle from start to current
        let mut width = (current.x - start.x).abs();
        let mut height = (current.y - start.y).abs();
        
        // Adjust dimensions to match aspect ratio
        if width / height > target_ratio {
            // Too wide, adjust width
            width = height * target_ratio;
        } else {
            // Too tall, adjust height  
            height = width / target_ratio;
        }
        
        // Determine the direction of the drag
        let center_x = (start.x + current.x) / 2.0;
        let center_y = (start.y + current.y) / 2.0;
        
        // Create rectangle centered on the drag center
        let half_width = width / 2.0;
        let half_height = height / 2.0;
        
        let mut min_x = center_x - half_width;
        let mut min_y = center_y - half_height;
        let mut max_x = center_x + half_width;
        let mut max_y = center_y + half_height;
        
        // Keep rectangle within screen bounds
        if min_x < screen_rect.min.x {
            let offset = screen_rect.min.x - min_x;
            min_x += offset;
            max_x += offset;
        }
        if max_x > screen_rect.max.x {
            let offset = max_x - screen_rect.max.x;
            min_x -= offset;
            max_x -= offset;
        }
        if min_y < screen_rect.min.y {
            let offset = screen_rect.min.y - min_y;
            min_y += offset;
            max_y += offset;
        }
        if max_y > screen_rect.max.y {
            let offset = max_y - screen_rect.max.y;
            min_y -= offset;
            max_y -= offset;
        }
        
        egui::Rect::from_min_max(
            egui::pos2(min_x, min_y),
            egui::pos2(max_x, max_y)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CropDragAction {
    Move,
    ResizeTopLeft,
    ResizeTopRight,
    ResizeBottomLeft,
    ResizeBottomRight,
}

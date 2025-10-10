use std::sync::Arc;
use std::time::Instant;
use eframe::egui;
use image;
use tokio::sync::RwLock;

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
    pub tint_enabled: bool, // Track tint toggle state separately from slider value
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
            tint_enabled: false,
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

        self.update_ui(ctx);
    }
}

#[allow(dead_code)]
enum Phase {
    Input,
    Editing,
}

impl PixelSorterApp {
    #[allow(dead_code)]
    fn render_input_phase(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let screen_rect = ui.max_rect();
        let button_height = 50.0;
        let button_area = egui::Rect::from_min_size(
            screen_rect.left_bottom() - egui::vec2(0.0, button_height),
            egui::vec2(screen_rect.width(), button_height),
        );

        // Display camera preview or placeholder
        if let Some(texture) = &self.camera_texture {
            ui.allocate_ui_at_rect(screen_rect.shrink2(egui::vec2(0.0, button_height)), |ui| {
                ui.add_sized(screen_rect.size(), egui::Image::new(texture));
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No camera available - Load an image to begin");
            });
        }

        // Buttons at the bottom
        ui.allocate_ui_at_rect(button_area, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Take Picture").clicked() {
                    self.capture_and_sort(ctx);
                }
                if ui.button("Upload Image").clicked() {
                    self.load_image(ctx);
                }
            });
        });
    }

    #[allow(dead_code)]
    fn render_editing_phase(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let screen_rect = ui.max_rect();
        let control_height = 100.0;
        let button_height = 50.0;
        
        // Rearranged layout: image at top, controls in middle, buttons at bottom
        let image_area = egui::Rect::from_min_size(
            screen_rect.min,
            egui::vec2(screen_rect.width(), screen_rect.height() - control_height - button_height),
        );
        let control_area = egui::Rect::from_min_size(
            screen_rect.min + egui::vec2(0.0, image_area.height()),
            egui::vec2(screen_rect.width(), control_height)
        );
        let button_area = egui::Rect::from_min_size(
            screen_rect.left_bottom() - egui::vec2(0.0, button_height),
            egui::vec2(screen_rect.width(), button_height),
        );

        // Display processed image at the top
        if let Some(texture) = self.processed_texture.clone() {
            ui.allocate_ui_at_rect(image_area, |ui| {
                ui.add_sized(image_area.size(), egui::Image::new(&texture));

                // Crop selection
                let response = ui.interact(image_area, egui::Id::new("crop_selection"), egui::Sense::click_and_drag());
                if response.clicked() || response.drag_started() {
                    self.selection_start = response.interact_pointer_pos();
                }
                if response.dragged() {
                    if let Some(start) = self.selection_start {
                        if let Some(current) = response.interact_pointer_pos() {
                            let _rect = egui::Rect::from_two_pos(start, current);
                            self.crop_rect = Some(self.constrain_crop_rect(start, current, image_area));
                        }
                    }
                }

                // Draw crop rect and handles
                if self.crop_mode && self.crop_rect.is_some() {
                    let painter = ui.painter();
                    if let Some(size) = self.processed_image.as_ref().map(|img| [img.width() as f32, img.height() as f32]) {
                        let display_rect = image_area;
                        let scale_x = display_rect.width() / size[0];
                        let scale_y = display_rect.height() / size[1];
                        let scale = scale_x.min(scale_y);
                        let offset_x = (display_rect.width() - size[0] * scale) / 2.0;
                        let offset_y = (display_rect.height() - size[1] * scale) / 2.0;
                        let crop_rect = self.crop_rect.unwrap();
                        let crop_min = display_rect.min + egui::vec2(offset_x + crop_rect.min.x * scale, offset_y + crop_rect.min.y * scale);
                        let crop_size = egui::vec2((crop_rect.max.x - crop_rect.min.x) * scale, (crop_rect.max.y - crop_rect.min.y) * scale);
                        let crop_display_rect = egui::Rect::from_min_size(crop_min, crop_size);
                        painter.rect_stroke(crop_display_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::RED));

                        // Add handles
                        let handle_size = 20.0;
                        let corners = [
                            ("tl", crop_display_rect.min),
                            ("tr", crop_display_rect.right_top()),
                            ("bl", crop_display_rect.left_bottom()),
                            ("br", crop_display_rect.right_bottom()),
                        ];
                        for (id, corner) in corners {
                            let handle_rect = egui::Rect::from_center_size(corner, egui::vec2(handle_size, handle_size));
                            let response = ui.interact(handle_rect, egui::Id::new(format!("crop_handle_{}", id)), egui::Sense::drag());
                            if response.dragged() {
                                if let Some(pos) = response.interact_pointer_pos() {
                                    let new_pos = (pos - display_rect.min - egui::vec2(offset_x, offset_y)) / scale;
                                    let mut new_rect = crop_rect;
                                    match id {
                                        "tl" => new_rect.min = egui::Pos2::new(new_pos.x, new_pos.y),
                                        "tr" => { new_rect.min.y = new_pos.y; new_rect.max.x = new_pos.x; }
                                        "bl" => { new_rect.min.x = new_pos.x; new_rect.max.y = new_pos.y; }
                                        "br" => new_rect.max = egui::Pos2::new(new_pos.x, new_pos.y),
                                        _ => {}
                                    }
                                    self.crop_rect = Some(self.constrain_crop_rect(new_rect.min, new_rect.max, display_rect));
                                }
                            }
                            painter.rect_filled(handle_rect, 0.0, egui::Color32::WHITE);
                        }
                    }
                }
            });
        } else {
            ui.allocate_ui_at_rect(image_area, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("No image to edit");
                });
            });
        }

        // Controls in the middle (above bottom buttons)
        ui.allocate_ui_at_rect(control_area, |ui| {
            ui.vertical(|ui| {
                if !self.crop_mode {
                    // Algorithm and parameters
                    ui.horizontal(|ui| {
                            ui.label("Algorithm:");
                            if ui.button(self.current_algorithm.name()).clicked() {
                                let all = SortingAlgorithm::all();
                                let idx = all.iter().position(|&a| a == self.current_algorithm).unwrap_or(0);
                                let next_idx = (idx + 1) % all.len();
                                self.current_algorithm = all[next_idx];
                                self.apply_pixel_sort(ctx);
                            }

                        ui.add_space(15.0);

                        // Color Tint Slider
                        // Tint toggle and slider
                        let mut tint_changed = false;
                        let mut tint_toggled = false;
                        ui.horizontal(|ui| {
                            if ui.button(if self.tint_enabled { "Tint: ON" } else { "Tint: OFF" }).clicked() {
                                self.tint_enabled = !self.tint_enabled;
                                tint_toggled = true;
                                if self.tint_enabled && self.sorting_params.color_tint == 0.0 {
                                    self.sorting_params.color_tint = 180.0; // default value when enabled
                                }
                            }
                            tint_changed = ui.add_enabled(
                                self.tint_enabled,
                                egui::Slider::new(&mut self.sorting_params.color_tint, 0.0..=360.0)
                                    .step_by(1.0)
                                    .show_value(false)
                            ).changed();
                        });

                        ui.add_space(10.0);

                        // Threshold Slider
                        ui.label(format!("Threshold: {:.0}", self.sorting_params.threshold));
                        let threshold_changed = ui.add(
                            egui::Slider::new(&mut self.sorting_params.threshold, 0.0..=255.0)
                                .step_by(1.0)
                                .show_value(false)
                        ).changed();

                        if (tint_changed || tint_toggled || threshold_changed) && !self.is_processing {
                            self.apply_pixel_sort(ctx);
                        }
                    });

                    ui.add_space(10.0);
                }

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
                        let aspect_changed = egui::ComboBox::from_id_source("crop_aspect_ratio")
                            .selected_text(self.crop_aspect_ratio.name())
                            .show_ui(ui, |ui| {
                                for &ratio in CropAspectRatio::all() {
                                    ui.selectable_value(&mut self.crop_aspect_ratio, ratio, ratio.name());
                                }
                            }).response.changed();
                        if aspect_changed {
                            if let Some(rect) = self.crop_rect {
                                self.crop_rect = Some(self.constrain_crop_rect(rect.min, rect.max, ctx.screen_rect()));
                            }
                        }

                        ui.separator();

                        if ui.button("Rotate 90°").clicked() {
                            self.crop_rotation = (self.crop_rotation + 90) % 360;
                        }
                    }

                    if self.crop_mode && self.crop_rect.is_some() {
                        ui.separator();
                        if ui.button("Apply Crop").clicked() {
                            self.apply_crop_and_sort(ctx);
                        }
                    }
                });
            });
        });

        // Main control buttons at the bottom
        ui.allocate_ui_at_rect(button_area, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Save & Continue").clicked() {
                    self.save_and_continue_iteration(ctx);
                }
                if ui.button("Take New Picture").clicked() {
                    self.preview_mode = true;
                    self.original_image = None;
                    self.processed_image = None;
                    self.processed_texture = None;
                    self.crop_mode = false;
                    self.crop_rect = None;
                }
            });
        });
    }

    #[allow(dead_code)]
    fn update_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.get_current_phase() {
                Phase::Input => self.render_input_phase(ui, ctx),
                Phase::Editing => self.render_editing_phase(ui, ctx),
            }
        });
    }

    #[allow(dead_code)]
    fn get_current_phase(&self) -> Phase {
        if self.original_image.is_some() || self.processed_image.is_some() {
            Phase::Editing
        } else {
            Phase::Input
        }
    }
}

impl PixelSorterApp {






}



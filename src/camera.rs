use crate::PixelSorterApp;
use eframe::egui;

impl PixelSorterApp {
    pub fn capture_and_sort(&mut self, ctx: &egui::Context) {
        if let Some(camera) = self.camera_controller.clone() {
            if let Ok(camera_lock) = camera.try_write() {
                if let Ok(frame) = camera_lock.capture_snapshot() {
                    self.original_image = Some(frame.clone());
                    self.processed_image = Some(frame.clone());
                    self.create_processed_texture(ctx, frame);
                    self.preview_mode = false;
                    self.current_phase = crate::ui::Phase::Edit; // Switch to edit phase
                }
            }
        }
    }
}
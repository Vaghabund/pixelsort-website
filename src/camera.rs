use crate::PixelSorterApp;
use eframe::egui;

impl PixelSorterApp {
    pub fn capture_and_sort(&mut self, ctx: &egui::Context) {
        if let Some(camera) = self.camera_controller.clone() {
            if let Ok(camera_lock) = camera.try_write() {
                match camera_lock.capture_snapshot() {
                    Ok(frame) => {
                        self.original_image = Some(frame.clone());
                        self.processed_image = Some(frame.clone());
                        self.create_processed_texture(ctx, frame);
                        self.preview_mode = false;
                        self.status_message = "Captured frame - Ready to edit!".to_string();
                    }
                    Err(e) => {
                        self.status_message = format!("Capture failed: {}", e);
                    }
                }
            } else {
                self.status_message = "Camera busy".to_string();
            }
        } else {
            self.status_message = "No camera available".to_string();
        }
    }
}
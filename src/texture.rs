use egui::{Context, TextureOptions};
use image::RgbImage;
use crate::PixelSorterApp;

impl PixelSorterApp {
    pub fn update_camera_texture(&mut self, ctx: &Context, image: &RgbImage) {
        // Validate image before updating to prevent white flash
        if image.width() == 0 || image.height() == 0 {
            return; // Skip invalid frames
        }
        
        let size = [image.width() as usize, image.height() as usize];
        let pixels = image.as_flat_samples();

        let color_image = egui::ColorImage::from_rgb(size, pixels.as_slice());

        // Reuse existing texture - optimized for 30 FPS updates
        match &mut self.camera_texture {
            Some(texture) => {
                // Only update if size matches to prevent flash
                if texture.size() == size {
                    texture.set(color_image, TextureOptions::NEAREST);
                } else {
                    // Size changed, recreate texture
                    *texture = ctx.load_texture("camera_preview", color_image, TextureOptions::NEAREST);
                }
            }
            None => {
                // First time only
                let texture = ctx.load_texture("camera_preview", color_image, TextureOptions::NEAREST);
                self.camera_texture = Some(texture);
            }
        }
    }

    pub fn create_processed_texture(&mut self, ctx: &Context, image: RgbImage) {
        let size = [image.width() as usize, image.height() as usize];
        let pixels = image.as_flat_samples();

        let color_image = egui::ColorImage::from_rgb(size, pixels.as_slice());

        // Reuse existing texture if available to reduce memory allocations
        match &mut self.processed_texture {
            Some(texture) => {
                // Update existing texture instead of creating new one
                texture.set(color_image, TextureOptions::NEAREST);
            }
            None => {
                let texture = ctx.load_texture("processed_image", color_image, TextureOptions::NEAREST);
                self.processed_texture = Some(texture);
            }
        }
    }
}
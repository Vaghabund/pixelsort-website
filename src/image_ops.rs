use crate::PixelSorterApp;
use eframe::egui;
use image;
use std::sync::Arc;

impl PixelSorterApp {
    pub fn apply_pixel_sort(&mut self, ctx: &egui::Context) {
        if let Some(ref original) = self.original_image.clone() {
            self.is_processing = true;
            self.status_message = format!("Applying {} sorting...", self.current_algorithm.name());

            let algorithm = self.current_algorithm;
            let params = self.sorting_params.clone();
            let pixel_sorter = Arc::clone(&self.pixel_sorter);

            match pixel_sorter.sort_pixels(&original, algorithm, &params) {
                Ok(mut sorted) => {
                    // Apply tint AFTER pixel sorting (as a visual effect only)
                    if self.tint_enabled && self.sorting_params.color_tint > 0.0 {
                        self.apply_tint_to_image(&mut sorted, self.sorting_params.color_tint);
                    }
                    
                    self.processed_image = Some(sorted.clone());
                    self.create_processed_texture(ctx, sorted);
                    self.is_processing = false;
                    self.status_message = "Sorting completed!".to_string();
                }
                Err(e) => {
                    self.is_processing = false;
                    self.status_message = format!("Processing failed: {}", e);
                }
            }
        } else {
            self.status_message = "No image to process".to_string();
        }
    }

    fn apply_tint_to_image(&self, image: &mut image::RgbImage, tint_hue: f32) {
        let (width, height) = image.dimensions();
        let tint_color = crate::pixel_sorter::hue_to_rgb_pixel(tint_hue);
        let strength = 0.2; // Strength for tinting
        
        for y in 0..height {
            for x in 0..width {
                let pixel = image.get_pixel(x, y);
                let tinted = self.blend_tint_preserve_luminance(pixel, &tint_color, strength);
                image.put_pixel(x, y, tinted);
            }
        }
    }

    fn blend_tint_preserve_luminance(&self, original: &image::Rgb<u8>, tint: &image::Rgb<u8>, strength: f32) -> image::Rgb<u8> {
        let strength = strength.clamp(0.0, 1.0);
        
        let orig_r = original[0] as f32 / 255.0;
        let orig_g = original[1] as f32 / 255.0;
        let orig_b = original[2] as f32 / 255.0;
        
        // Calculate luminance to preserve brightness
        let luminance = 0.299 * orig_r + 0.587 * orig_g + 0.114 * orig_b;
        
        // For very dark or very bright pixels, reduce tint strength
        let adjusted_strength = if luminance < 0.1 || luminance > 0.9 {
            strength * 0.3  // Preserve blacks and whites more
        } else {
            strength
        };
        
        let tint_r = tint[0] as f32 / 255.0;
        let tint_g = tint[1] as f32 / 255.0;
        let tint_b = tint[2] as f32 / 255.0;
        
        // Blend with original
        let final_r = (orig_r * (1.0 - adjusted_strength) + orig_r * tint_r * adjusted_strength).clamp(0.0, 1.0);
        let final_g = (orig_g * (1.0 - adjusted_strength) + orig_g * tint_g * adjusted_strength).clamp(0.0, 1.0);
        let final_b = (orig_b * (1.0 - adjusted_strength) + orig_b * tint_b * adjusted_strength).clamp(0.0, 1.0);
        
        image::Rgb([
            (final_r * 255.0).round() as u8,
            (final_g * 255.0).round() as u8,
            (final_b * 255.0).round() as u8,
        ])
    }


    pub fn load_image(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Image Files", &["png", "jpg", "jpeg", "bmp", "tiff"])
            .pick_file()
        {
            match image::open(&path) {
                Ok(img) => {
                    let rgb_image = img.to_rgb8();
                    self.original_image = Some(rgb_image.clone());
                    self.processed_image = Some(rgb_image.clone());
                    self.create_processed_texture(ctx, rgb_image);
                    self.status_message = format!("Loaded image: {}", path.display());
                    self.preview_mode = false;
                }
                Err(e) => {
                    self.status_message = format!("Failed to load image: {}", e);
                }
            }
        }
    }

        // Removed unused method save_image
}
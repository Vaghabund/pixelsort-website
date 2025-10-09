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
                Ok(sorted) => {
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

    pub fn save_image(&mut self) {
        if let Some(ref processed) = self.processed_image {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG Files", &["png"])
                .save_file()
            {
                match processed.save(&path) {
                    Ok(_) => {
                        self.status_message = format!("Saved image: {}", path.display());
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to save image: {}", e);
                    }
                }
            }
        } else {
            self.status_message = "No image to save".to_string();
        }
    }
}
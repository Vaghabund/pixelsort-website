use std::sync::Arc;
use crate::PixelSorterApp;

impl PixelSorterApp {
    pub fn apply_crop_and_sort(&mut self, ctx: &egui::Context) {
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

    pub fn constrain_crop_rect(&self, start: egui::Pos2, current: egui::Pos2, _screen_rect: egui::Rect) -> egui::Rect {
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

        egui::Rect {
            min: egui::Pos2::new(center_x - half_width, center_y - half_height),
            max: egui::Pos2::new(center_x + half_width, center_y + half_height),
        }
    }
}
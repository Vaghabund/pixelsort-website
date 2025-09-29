use anyhow::Result;
use eframe::egui;
use log::info;
use std::sync::Arc;
use tokio::sync::RwLock;

mod config;
mod gpio_controller;
mod image_processor;
mod pixel_sorter;
mod ui;

use crate::config::Config;
use crate::gpio_controller::GpioController;
use crate::image_processor::ImageProcessor;
use crate::pixel_sorter::PixelSorter;
use crate::ui::PixelSorterApp;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    info!("🎨 Starting Raspberry Pi Pixel Sorter (Rust Edition)");

    // Load configuration
    let config = Config::load()?;
    info!("Configuration loaded: {}x{} display", config.display.width, config.display.height);

    // Initialize components
    let pixel_sorter = Arc::new(PixelSorter::new());
    let image_processor = Arc::new(RwLock::new(ImageProcessor::new()));
    
    // Initialize GPIO controller
    let gpio_controller = match GpioController::new().await {
        Ok(controller) => {
            info!("GPIO controller initialized successfully");
            Some(Arc::new(RwLock::new(controller)))
        }
        Err(e) => {
            log::warn!("GPIO initialization failed: {}. Running in simulation mode.", e);
            None
        }
    };

    // Setup eframe options for 1600x860 fullscreen display
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 860.0])    // Fixed size 1600x860
            .with_min_inner_size([1600.0, 860.0]) // Lock minimum size
            .with_max_inner_size([1600.0, 860.0]) // Lock maximum size
            .with_decorations(false)              // No window decorations
            .with_resizable(false)                // Not resizable
            .with_maximized(true)                 // Start maximized
            .with_fullscreen(true)                // Force fullscreen mode
            .with_always_on_top(),                // Keep on top
        ..Default::default()
    };

    info!("Launching GUI application...");

    // Run the application
    eframe::run_native(
        "Raspberry Pi Pixel Sorter",
        options,
        Box::new(|cc| {
            // Setup egui style for touch interface
            setup_touch_style(&cc.egui_ctx);
            
            Box::new(PixelSorterApp::new(
                pixel_sorter,
                image_processor,
                gpio_controller,
                config,
            ))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run application: {}", e))?;

    info!("Application shut down gracefully");
    Ok(())
}

fn setup_touch_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    // Larger UI elements for touch interaction
    style.spacing.button_padding = egui::vec2(16.0, 12.0);
    style.spacing.item_spacing = egui::vec2(12.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(16.0);
    style.spacing.menu_margin = egui::Margin::same(8.0);
    
    // Larger text for better readability
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(18.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(16.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(24.0, egui::FontFamily::Proportional),
    );
    
    // Touch-friendly slider and other controls
    style.spacing.slider_width = 300.0;
    style.spacing.combo_width = 200.0;
    
    ctx.set_style(style);
}
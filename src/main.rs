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

    // KIOSK MODE: Borderless window that fills the entire screen
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 600.0])    // Exact screen size
            .with_position([0.0, 0.0])            // Top-left corner (no gaps)
            .with_decorations(false)              // NO title bar, borders, or buttons
            .with_resizable(false)                // Cannot be resized
            .with_movable(false)                  // Cannot be moved
            .with_close_button(false)             // No close button
            .with_minimize_button(false)          // No minimize button
            .with_maximize_button(false),         // No maximize button
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
    
    // Kiosk-style UI with minimal margins
    style.spacing.button_padding = egui::vec2(16.0, 12.0);
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(4.0);  // Minimal margins
    style.spacing.menu_margin = egui::Margin::same(4.0);
    
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
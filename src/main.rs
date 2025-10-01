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
mod camera_controller;

use crate::config::Config;
use crate::gpio_controller::GpioController;
use crate::image_processor::ImageProcessor;
use crate::pixel_sorter::PixelSorter;
use crate::ui::PixelSorterApp;
use crate::camera_controller::CameraController;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up detailed logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    info!("🎨 Starting Raspberry Pi Pixel Sorter (Rust Edition)");
    println!("🔍 DEBUG logging enabled - you should see detailed camera debug messages");

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

    // Initialize Camera controller  
    info!("🔍 DEBUG: Attempting to initialize camera controller...");
    let camera_controller = match CameraController::new() {
        Ok(controller) => {
            info!("✅ DEBUG: Camera controller initialized successfully");
            info!("🔍 DEBUG: Camera available: {}", controller.is_available());
            Some(Arc::new(RwLock::new(controller)))
        }
        Err(e) => {
            log::error!("❌ DEBUG: Camera initialization failed: {}. Camera features disabled.", e);
            None
        }
    };

    // WINDOWED MODE: Normal window with decorations for manual resizing
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])     // Initial size (can be resized)
            .with_min_inner_size([400.0, 300.0]) // Minimum size
            .with_decorations(true)               // Show title bar, borders, and buttons
            .with_resizable(true)                 // Can be resized
            .with_close_button(true)              // Show close button
            .with_minimize_button(true)           // Show minimize button
            .with_maximize_button(true),          // Show maximize button
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
                camera_controller,
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
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub display: DisplayConfig,
    pub gpio: GpioConfig,
    pub processing: ProcessingConfig,
    pub paths: PathConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub image_display_width: u32,
    pub image_display_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpioConfig {
    pub enabled: bool,
    pub debounce_ms: u64,
    pub pins: GpioPins,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpioPins {
    pub load_image: u8,
    pub next_algorithm: u8,
    pub threshold_up: u8,
    pub threshold_down: u8,
    pub save_image: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    pub default_threshold: f32,
    pub default_interval: usize,
    pub max_image_width: u32,
    pub max_image_height: u32,
    pub preview_scale_factor: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    pub sample_images_dir: PathBuf,
    pub default_save_dir: PathBuf,
    pub config_file: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            display: DisplayConfig {
                width: 800,
                height: 480,
                fullscreen: true,
                image_display_width: 480,
                image_display_height: 360,
            },
            gpio: GpioConfig {
                enabled: true,
                debounce_ms: 200,
                pins: GpioPins {
                    load_image: 18,
                    next_algorithm: 19,
                    threshold_up: 20,
                    threshold_down: 21,
                    save_image: 26,
                },
            },
            processing: ProcessingConfig {
                default_threshold: 50.0,
                default_interval: 10,
                max_image_width: 1920,
                max_image_height: 1080,
                preview_scale_factor: 4,
            },
            paths: PathConfig {
                sample_images_dir: PathBuf::from("sample_images"),
                default_save_dir: PathBuf::from("output"),
                config_file: PathBuf::from("pixelsort_config.toml"),
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = PathBuf::from("pixelsort_config.toml");
        
        if config_path.exists() {
            Self::load_from_file(&config_path)
        } else {
            log::info!("Config file not found, creating default configuration");
            let default_config = Self::default();
            default_config.save()?;
            Ok(default_config)
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        
        let config: Self = toml::from_str(&contents)
            .with_context(|| "Failed to parse configuration file")?;
        
        log::info!("Configuration loaded from {}", path.as_ref().display());
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to_file(&self.paths.config_file)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
        
        // Ensure parent directory exists
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        std::fs::write(path.as_ref(), contents)
            .with_context(|| format!("Failed to write config file: {}", path.as_ref().display()))?;
        
        log::info!("Configuration saved to {}", path.as_ref().display());
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        // Validate display settings
        if self.display.width == 0 || self.display.height == 0 {
            return Err(anyhow::anyhow!("Invalid display dimensions"));
        }

        if self.display.image_display_width > self.display.width || 
           self.display.image_display_height > self.display.height {
            return Err(anyhow::anyhow!("Image display area larger than screen"));
        }

        // Validate processing settings
        if self.processing.default_threshold < 0.0 || self.processing.default_threshold > 255.0 {
            return Err(anyhow::anyhow!("Invalid default threshold"));
        }

        if self.processing.default_interval == 0 {
            return Err(anyhow::anyhow!("Invalid default interval"));
        }

        // Validate GPIO pins don't conflict
        let pins = vec![
            self.gpio.pins.load_image,
            self.gpio.pins.next_algorithm,
            self.gpio.pins.threshold_up,
            self.gpio.pins.threshold_down,
            self.gpio.pins.save_image,
        ];

        for (i, &pin1) in pins.iter().enumerate() {
            for &pin2 in pins.iter().skip(i + 1) {
                if pin1 == pin2 {
                    return Err(anyhow::anyhow!("Duplicate GPIO pin assignment: {}", pin1));
                }
            }
        }

        Ok(())
    }

    pub fn create_directories(&self) -> Result<()> {
        // Create sample images directory
        std::fs::create_dir_all(&self.paths.sample_images_dir)
            .with_context(|| format!("Failed to create sample images directory: {}", 
                self.paths.sample_images_dir.display()))?;

        // Create default save directory
        std::fs::create_dir_all(&self.paths.default_save_dir)
            .with_context(|| format!("Failed to create save directory: {}", 
                self.paths.default_save_dir.display()))?;

        log::info!("Created necessary directories");
        Ok(())
    }

    // Helper methods for common operations
    pub fn get_display_aspect_ratio(&self) -> f32 {
        self.display.width as f32 / self.display.height as f32
    }

    pub fn get_image_display_aspect_ratio(&self) -> f32 {
        self.display.image_display_width as f32 / self.display.image_display_height as f32
    }

    pub fn is_raspberry_pi_resolution(&self) -> bool {
        // Common Raspberry Pi display resolutions
        matches!(
            (self.display.width, self.display.height),
            (800, 480) | (1024, 600) | (1280, 720) | (1920, 1080)
        )
    }

    pub fn update_display_size(&mut self, width: u32, height: u32) -> Result<()> {
        if width == 0 || height == 0 {
            return Err(anyhow::anyhow!("Invalid display dimensions: {}x{}", width, height));
        }

        self.display.width = width;
        self.display.height = height;
        
        // Automatically adjust image display area to maintain reasonable proportions
        self.display.image_display_width = (width as f32 * 0.6) as u32;
        self.display.image_display_height = (height as f32 * 0.75) as u32;
        
        log::info!("Updated display configuration to {}x{}", width, height);
        Ok(())
    }
}

// Configuration builder for easier setup
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    pub fn display_size(mut self, width: u32, height: u32) -> Self {
        self.config.display.width = width;
        self.config.display.height = height;
        self
    }

    pub fn fullscreen(mut self, enabled: bool) -> Self {
        self.config.display.fullscreen = enabled;
        self
    }

    pub fn gpio_enabled(mut self, enabled: bool) -> Self {
        self.config.gpio.enabled = enabled;
        self
    }

    pub fn gpio_pin(mut self, function: &str, pin: u8) -> Self {
        match function {
            "load_image" => self.config.gpio.pins.load_image = pin,
            "next_algorithm" => self.config.gpio.pins.next_algorithm = pin,
            "threshold_up" => self.config.gpio.pins.threshold_up = pin,
            "threshold_down" => self.config.gpio.pins.threshold_down = pin,
            "save_image" => self.config.gpio.pins.save_image = pin,
            _ => log::warn!("Unknown GPIO function: {}", function),
        }
        self
    }

    pub fn max_image_size(mut self, width: u32, height: u32) -> Self {
        self.config.processing.max_image_width = width;
        self.config.processing.max_image_height = height;
        self
    }

    pub fn default_threshold(mut self, threshold: f32) -> Self {
        self.config.processing.default_threshold = threshold;
        self
    }

    pub fn build(self) -> Result<Config> {
        self.config.validate()?;
        Ok(self.config)
    }
}

// Environment-specific configuration presets
impl Config {
    pub fn raspberry_pi_7inch() -> Self {
        Config {
            display: DisplayConfig {
                width: 800,
                height: 480,
                fullscreen: true,
                image_display_width: 480,
                image_display_height: 360,
            },
            ..Default::default()
        }
    }

    pub fn development_desktop() -> Self {
        Config {
            display: DisplayConfig {
                width: 1024,
                height: 768,
                fullscreen: false,
                image_display_width: 600,
                image_display_height: 450,
            },
            gpio: GpioConfig {
                enabled: false,
                ..Default::default().gpio
            },
            ..Default::default()
        }
    }

    pub fn raspberry_pi_hdmi() -> Self {
        Config {
            display: DisplayConfig {
                width: 1920,
                height: 1080,
                fullscreen: true,
                image_display_width: 1200,
                image_display_height: 900,
            },
            processing: ProcessingConfig {
                max_image_width: 2560,
                max_image_height: 1440,
                ..Default::default().processing
            },
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
        assert!(config.is_raspberry_pi_resolution());
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .display_size(1024, 768)
            .fullscreen(false)
            .gpio_enabled(false)
            .default_threshold(75.0)
            .build()
            .unwrap();

        assert_eq!(config.display.width, 1024);
        assert_eq!(config.display.height, 768);
        assert!(!config.display.fullscreen);
        assert!(!config.gpio.enabled);
        assert_eq!(config.processing.default_threshold, 75.0);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Test invalid threshold
        config.processing.default_threshold = 300.0;
        assert!(config.validate().is_err());
        
        // Test duplicate GPIO pins
        config.processing.default_threshold = 50.0;
        config.gpio.pins.load_image = 18;
        config.gpio.pins.next_algorithm = 18; // Duplicate
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let original_config = Config::raspberry_pi_7inch();
        original_config.save_to_file(&config_path).unwrap();
        
        let loaded_config = Config::load_from_file(&config_path).unwrap();
        
        assert_eq!(original_config.display.width, loaded_config.display.width);
        assert_eq!(original_config.display.height, loaded_config.display.height);
        assert_eq!(original_config.gpio.pins.load_image, loaded_config.gpio.pins.load_image);
    }

    #[test]
    fn test_preset_configs() {
        assert!(Config::raspberry_pi_7inch().validate().is_ok());
        assert!(Config::development_desktop().validate().is_ok());
        assert!(Config::raspberry_pi_hdmi().validate().is_ok());
    }
}
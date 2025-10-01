use anyhow::{Context, Result};
use rppal::gpio::{Gpio, InputPin, Level, Trigger};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{Duration, Instant};
use log::{info, error};

#[derive(Debug, Clone, Copy)]
pub struct ButtonConfig {
    pub pin: u8,
    pub function: ButtonFunction,
}

#[derive(Debug, Clone, Copy)]
pub enum ButtonFunction {
    LoadImage = 1,
    NextAlgorithm = 2,
    ThresholdUp = 3,
    ThresholdDown = 4,
    SaveImage = 5,
}

impl ButtonFunction {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(ButtonFunction::LoadImage),
            2 => Some(ButtonFunction::NextAlgorithm),
            3 => Some(ButtonFunction::ThresholdUp),
            4 => Some(ButtonFunction::ThresholdDown),
            5 => Some(ButtonFunction::SaveImage),
            _ => None,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ButtonFunction::LoadImage => "Load new image",
            ButtonFunction::NextAlgorithm => "Next algorithm",
            ButtonFunction::ThresholdUp => "Increase threshold",
            ButtonFunction::ThresholdDown => "Decrease threshold",
            ButtonFunction::SaveImage => "Save image",
        }
    }
}

pub struct GpioController {
    _gpio: Gpio,
    buttons: HashMap<u8, ButtonConfig>,
    button_sender: mpsc::UnboundedSender<u8>,
    button_receiver: Arc<RwLock<mpsc::UnboundedReceiver<u8>>>,
    last_press_times: Arc<RwLock<HashMap<u8, Instant>>>,
    debounce_duration: Duration,
}

impl GpioController {
    pub async fn new() -> Result<Self> {
        let gpio = Gpio::new().context("Failed to initialize GPIO")?;
        
        // Default button configuration for Raspberry Pi
        let button_configs = vec![
            ButtonConfig { pin: 18, function: ButtonFunction::LoadImage },
            ButtonConfig { pin: 19, function: ButtonFunction::NextAlgorithm },
            ButtonConfig { pin: 20, function: ButtonFunction::ThresholdUp },
            ButtonConfig { pin: 21, function: ButtonFunction::ThresholdDown },
            ButtonConfig { pin: 26, function: ButtonFunction::SaveImage },
        ];

        let mut buttons = HashMap::new();
        let (button_sender, button_receiver) = mpsc::unbounded_channel();
        let last_press_times = Arc::new(RwLock::new(HashMap::new()));
        
        // Setup GPIO pins
        for config in button_configs {
            buttons.insert(config.pin, config);
            
            let pin = gpio.get(config.pin)
                .context(format!("Failed to get GPIO pin {}", config.pin))?
                .into_input_pullup();

            // Setup interrupt for button press
            let sender = button_sender.clone();
            let last_times = Arc::clone(&last_press_times);
            let debounce_dur = Duration::from_millis(200);

            // Spawn a task to handle this button
            let button_id = config.function as u8;
            tokio::spawn(async move {
                Self::handle_button_interrupt(pin, button_id, sender, last_times, debounce_dur).await;
            });

            info!("GPIO pin {} configured for {}", config.pin, config.function.description());
        }

        Ok(Self {
            _gpio: gpio,
            buttons,
            button_sender,
            button_receiver: Arc::new(RwLock::new(button_receiver)),
            last_press_times,
            debounce_duration: Duration::from_millis(200),
        })
    }

    async fn handle_button_interrupt(
        mut pin: InputPin,
        button_id: u8,
        sender: mpsc::UnboundedSender<u8>,
        last_press_times: Arc<RwLock<HashMap<u8, Instant>>>,
        debounce_duration: Duration,
    ) {
        // Set up interrupt on falling edge (button press)
        if let Err(e) = pin.set_async_interrupt(Trigger::FallingEdge, move |level| {
            if level == Level::Low {
                let sender = sender.clone();
                let last_times = Arc::clone(&last_press_times);
                let debounce_dur = debounce_duration;
                
                tokio::spawn(async move {
                    // Check debounce
                    let now = Instant::now();
                    let mut times = last_times.write().await;
                    
                    if let Some(&last_time) = times.get(&button_id) {
                        if now.duration_since(last_time) < debounce_dur {
                            return; // Too soon, ignore this press
                        }
                    }
                    
                    times.insert(button_id, now);
                    drop(times); // Release the lock
                    
                    // Send button press event
                    if let Err(e) = sender.send(button_id) {
                        error!("Failed to send button press event: {}", e);
                    } else {
                        info!("Button {} pressed", button_id);
                    }
                });
            }
        }) {
            error!("Failed to set up interrupt for button {}: {}", button_id, e);
        }

        // Keep the pin alive
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    pub async fn get_button_press(&self) -> Option<u8> {
        let mut receiver = self.button_receiver.write().await;
        receiver.try_recv().ok()
    }

    pub async fn wait_for_button_press(&self) -> Option<u8> {
        let mut receiver = self.button_receiver.write().await;
        receiver.recv().await
    }

    pub fn get_button_config(&self, pin: u8) -> Option<&ButtonConfig> {
        self.buttons.get(&pin)
    }

    pub fn get_all_buttons(&self) -> Vec<&ButtonConfig> {
        self.buttons.values().collect()
    }

    pub async fn test_buttons(&self, duration: Duration) -> Result<()> {
        info!("Testing GPIO buttons for {} seconds...", duration.as_secs());
        
        let start_time = Instant::now();
        
        while start_time.elapsed() < duration {
            if let Some(button_id) = self.get_button_press().await {
                if let Some(function) = ButtonFunction::from_id(button_id) {
                    info!("Button test: {} - {}", button_id, function.description());
                }
            }
            
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        info!("GPIO button test completed");
        Ok(())
    }
}

// Keyboard simulation for development on non-Raspberry Pi systems
#[cfg(not(target_arch = "aarch64"))]
pub struct KeyboardSimulator {
    button_sender: mpsc::UnboundedSender<u8>,
    button_receiver: Arc<RwLock<mpsc::UnboundedReceiver<u8>>>,
}

#[cfg(not(target_arch = "aarch64"))]
impl KeyboardSimulator {
    pub fn new() -> Self {
        let (button_sender, button_receiver) = mpsc::unbounded_channel();
        
        info!("Keyboard simulation active:");
        info!("  1: Load Image");
        info!("  2: Next Algorithm");
        info!("  3: Threshold Up");
        info!("  4: Threshold Down");
        info!("  5: Save Image");
        info!("  ESC: Exit");
        
        Self {
            button_sender,
            button_receiver: Arc::new(RwLock::new(button_receiver)),
        }
    }

    pub fn simulate_button_press(&self, button_id: u8) -> Result<()> {
        if ButtonFunction::from_id(button_id).is_some() {
            self.button_sender.send(button_id)
                .context("Failed to send simulated button press")?;
            info!("Simulated button press: {}", button_id);
        }
        Ok(())
    }

    pub async fn get_button_press(&self) -> Option<u8> {
        let mut receiver = self.button_receiver.write().await;
        receiver.try_recv().ok()
    }
}

// Factory function that returns appropriate controller based on platform
pub async fn create_controller() -> Result<Box<dyn ButtonController>> {
    #[cfg(target_arch = "aarch64")]
    {
        Ok(Box::new(GpioController::new().await?))
    }
    
    #[cfg(not(target_arch = "aarch64"))]
    {
        warn!("Not running on ARM64, using keyboard simulation");
        Ok(Box::new(KeyboardSimulator::new()))
    }
}

// Trait for button input abstraction
#[async_trait::async_trait]
pub trait ButtonController: Send + Sync {
    async fn get_button_press(&self) -> Option<u8>;
    async fn wait_for_button_press(&self) -> Option<u8>;
}

#[async_trait::async_trait]
impl ButtonController for GpioController {
    async fn get_button_press(&self) -> Option<u8> {
        self.get_button_press().await
    }

    async fn wait_for_button_press(&self) -> Option<u8> {
        self.wait_for_button_press().await
    }
}

#[cfg(not(target_arch = "aarch64"))]
#[async_trait::async_trait]
impl ButtonController for KeyboardSimulator {
    async fn get_button_press(&self) -> Option<u8> {
        self.get_button_press().await
    }

    async fn wait_for_button_press(&self) -> Option<u8> {
        let mut receiver = self.button_receiver.write().await;
        receiver.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_function_from_id() {
        assert!(matches!(ButtonFunction::from_id(1), Some(ButtonFunction::LoadImage)));
        assert!(matches!(ButtonFunction::from_id(5), Some(ButtonFunction::SaveImage)));
        assert!(ButtonFunction::from_id(0).is_none());
        assert!(ButtonFunction::from_id(6).is_none());
    }

    #[test]
    fn test_button_descriptions() {
        assert_eq!(ButtonFunction::LoadImage.description(), "Load new image");
        assert_eq!(ButtonFunction::SaveImage.description(), "Save image");
    }

    #[tokio::test]
    async fn test_keyboard_simulator() {
        #[cfg(not(target_arch = "aarch64"))]
        {
            let sim = KeyboardSimulator::new();
            sim.simulate_button_press(1).unwrap();
            
            let button = sim.get_button_press().await;
            assert_eq!(button, Some(1));
        }
    }
}
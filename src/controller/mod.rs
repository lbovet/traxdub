pub mod init;
pub mod driver;

use crate::engine::Engine;
use crate::ui::UI;
use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Base MIDI control assignments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseControlConfig {
    pub main_knob: MidiAssignment,
    pub secondary_knob: MidiAssignment,
    pub selection_button: MidiAssignment,
    pub back_button: MidiAssignment,
}

/// MIDI assignment for a control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiAssignment {
    pub channel: u8,
    pub control: u8,
    pub control_type: ControlType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlType {
    Knob,
    Button,
}

/// Navigation level
#[derive(Debug, Clone, PartialEq)]
pub enum NavigationLevel {
    Main,
    Secondary,
}

/// Navigation direction
#[derive(Debug, Clone, PartialEq)]
pub enum NavigationDirection {
    Backward,
    Forward,
}

/// Controller state machine states
#[derive(Debug, Clone, PartialEq)]
pub enum ControllerState {
    Initializing,
    LearningSelectionKnob,
    LearningSecondaryKnob,
    LearningSelectionButton,
    LearningBackButton,
    Navigating,
}

/// Main controller that processes MIDI events and coordinates engine and UI
pub struct Controller {
    ui: Arc<UI>,
    engine: Arc<Engine>,
    state: ControllerState,
    base_control_config: Option<BaseControlConfig>,
    config_path: PathBuf,
    force_init: bool,
}

impl Controller {
    /// Create a new controller instance
    pub fn new(ui: Arc<UI>, engine: Arc<Engine>, force_init: bool) -> Result<Self> {
        let config_path = Self::get_config_path();
        
        let mut controller = Self {
            ui,
            engine,
            state: ControllerState::Initializing,
            base_control_config: None,
            config_path,
            force_init,
        };
        
        controller.initialize()?;
        Ok(controller)
    }
    
    /// Get the configuration file path
    fn get_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let mut path = PathBuf::from(home);
        path.push(".traxdub");
        
        // Create directory if it doesn't exist
        if !path.exists() {
            fs::create_dir_all(&path).ok();
        }
        
        path.push("base-control.json");
        path
    }
    
    /// Initialize the controller
    fn initialize(&mut self) -> Result<()> {
        info!("Initializing controller...");
        
        // Force learning mode if --init flag is set
        if self.force_init {
            info!("Force initialization requested, entering learning mode");
            // Delete existing config if present
            if self.config_path.exists() {
                info!("Removing existing configuration file");
                fs::remove_file(&self.config_path).ok();
            }
            self.state = ControllerState::LearningSelectionKnob;
            self.start_learning_mode()?;
        } else if self.config_path.exists() {
            // Try to load existing config
            info!("Loading existing configuration from {:?}", self.config_path);
            self.load_config()?;
            self.state = ControllerState::Navigating;
        } else {
            info!("No configuration found, entering learning mode");
            self.state = ControllerState::LearningSelectionKnob;
            self.start_learning_mode()?;
        }
        
        Ok(())
    }
    
    /// Load configuration from file
    fn load_config(&mut self) -> Result<()> {
        let content = fs::read_to_string(&self.config_path)
            .context("Failed to read config file")?;
        let config: BaseControlConfig = serde_json::from_str(&content)
            .context("Failed to parse config file")?;
        self.base_control_config = Some(config);
        Ok(())
    }
    
    /// Save configuration to file
    fn save_config(&self) -> Result<()> {
        if let Some(config) = &self.base_control_config {
            let content = serde_json::to_string_pretty(config)?;
            fs::write(&self.config_path, content)
                .context("Failed to write config file")?;
            info!("Configuration saved to {:?}", self.config_path);
        }
        Ok(())
    }
    
    /// Process a MIDI event
    pub fn process_midi_event(&mut self, event: driver::MidiEvent) -> Result<()> {
        debug!("Processing MIDI event: {:?} in state {:?}", event, self.state);
        
        match self.state {
            ControllerState::LearningSelectionKnob => {
                self.learn_main_knob(event)?;
            }
            ControllerState::LearningSecondaryKnob => {
                self.learn_secondary_knob(event)?;
            }
            ControllerState::LearningSelectionButton => {
                self.learn_selection_button(event)?;
            }
            ControllerState::LearningBackButton => {
                self.learn_back_button(event)?;
            }
            ControllerState::Navigating => {
                self.process_event_navigating_state(event)?;
            }
            _ => {
                warn!("Received MIDI event in unexpected state: {:?}", self.state);
            }
        }
        
        Ok(())
    }
    
    /// Process events when in navigating state
    fn process_event_navigating_state(&mut self, event: driver::MidiEvent) -> Result<()> {
        if let driver::MidiEvent::ControlChange { channel, control, value } = event {
            if let Some(config) = &self.base_control_config {
                // Check if it's the main knob
                if config.main_knob.channel == channel && config.main_knob.control == control {
                    let direction = if value >= 64 {
                        NavigationDirection::Forward
                    } else {
                        NavigationDirection::Backward
                    };
                    debug!("Main knob navigation: {:?}, value: {}", direction, value);
                    self.ui.navigate(NavigationLevel::Main, direction)?;
                }
                // Check if it's the secondary knob
                else if config.secondary_knob.channel == channel && config.secondary_knob.control == control {
                    let direction = if value >= 64 {
                        NavigationDirection::Forward
                    } else {
                        NavigationDirection::Backward
                    };
                    debug!("Secondary knob navigation: {:?}, value: {}", direction, value);
                    self.ui.navigate(NavigationLevel::Secondary, direction)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Update the UI state
    pub fn update_ui_state(&self, state: &str) -> Result<()> {
        self.ui.signal_state_change(state)
    }
    
    /// Run loop with signal handling for graceful shutdown
    pub fn run_until_signal(&mut self, running: Arc<AtomicBool>) -> Result<()> {
        info!("Controller running in state: {:?}", self.state);
        
        // Start MIDI receiver and get the event channel
        info!("Starting MIDI event loop...");
        let (driver, event_receiver) = driver::Driver::start()?;
        
        driver.connect_all_midi_inputs()?;
        
        // Process events from the receiver until signal
        while running.load(Ordering::SeqCst) {
            match event_receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    debug!("Received MIDI event: {:?}", event);
                    if let Err(e) = self.process_midi_event(event) {
                        warn!("Error processing MIDI event: {}", e);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // No event received, continue loop to check signal
                    continue;
                }
                Err(e) => {
                    error!("Event receiver error: {}", e);
                    break;
                }
            }
        }
        
        info!("Controller shutting down gracefully");
        drop(driver);
        info!("MIDI receiver dropped");
        Ok(())
    }
}

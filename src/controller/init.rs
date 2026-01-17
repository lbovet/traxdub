use super::{BaseControlConfig, ControlType, Controller, ControllerState, MidiAssignment};
use crate::controller::driver;
use anyhow::{Context, Result};
use log::{debug, info};
use std::fs;
use std::path::PathBuf;

impl Controller {
    /// Get the configuration file path
    pub(super) fn get_config_path() -> PathBuf {
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
    pub(super) fn initialize(&mut self) -> Result<()> {
        debug!("Initializing controller...");
        
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
    pub(super) fn save_config(&self) -> Result<()> {
        if let Some(config) = &self.base_control_config {
            let content = serde_json::to_string_pretty(config)?;
            fs::write(&self.config_path, content)
                .context("Failed to write config file")?;
            info!("Configuration saved to {:?}", self.config_path);
        }
        Ok(())
    }
    /// Start the learning mode for base controls
    pub(super) fn start_learning_mode(&mut self) -> Result<()> {
        self.ui.prompt_turn_selection_knob()?;
        Ok(())
    }

    /// Learn the main knob assignment
    pub(super) fn learn_main_knob(&mut self, event: driver::MidiEvent) -> Result<()> {
        if let driver::MidiEvent::ControlChange { channel, control, .. } = event {
            info!("Learned selection knob: channel={}, cc={}", channel, control);

            let assignment = MidiAssignment {
                channel,
                control,
                control_type: ControlType::Knob,
            };

            // Initialize config or update
            if self.base_control_config.is_none() {
                self.base_control_config = Some(BaseControlConfig {
                    main_knob: assignment.clone(),
                    secondary_knob: assignment.clone(), // Placeholder
                    selection_button: assignment.clone(), // Placeholder
                    back_button: assignment,              // Placeholder
                });
            } else if let Some(config) = &mut self.base_control_config {
                config.main_knob = assignment;
            }

            // Move to next learning state
            self.state = ControllerState::LearningSecondaryKnob;
            self.ui.prompt_turn_secondary_knob()?;
        }

        Ok(())
    }

    /// Learn the secondary knob assignment
    pub(super) fn learn_secondary_knob(&mut self, event: driver::MidiEvent) -> Result<()> {
        if let driver::MidiEvent::ControlChange { channel, control, .. } = event {
            // Ignore if this is the already-learned main knob
            if let Some(config) = &self.base_control_config {
                if config.main_knob.channel == channel && config.main_knob.control == control {
                    debug!("Ignoring main knob event during secondary knob learning");
                    return Ok(());
                }
            }

            info!(
                "Learned secondary knob: channel={}, cc={}",
                channel, control
            );

            if let Some(config) = &mut self.base_control_config {
                config.secondary_knob = MidiAssignment {
                    channel,
                    control,
                    control_type: ControlType::Knob,
                };
            }

            // Move to next learning state
            self.state = ControllerState::LearningSelectionButton;
            self.ui.prompt_press_selection_button()?;
        }

        Ok(())
    }

    /// Learn the selection button assignment
    pub(super) fn learn_selection_button(&mut self, event: driver::MidiEvent) -> Result<()> {
        if let driver::MidiEvent::ControlChange { channel, control, .. } = event {
            // Ignore if this is an already-learned control
            if let Some(config) = &self.base_control_config {
                if (config.main_knob.channel == channel && config.main_knob.control == control)
                    || (config.secondary_knob.channel == channel
                        && config.secondary_knob.control == control)
                {
                    debug!("Ignoring already-learned knob event during button learning");
                    return Ok(());
                }
            }

            info!(
                "Learned selection button: channel={}, cc={}",
                channel, control
            );

            if let Some(config) = &mut self.base_control_config {
                config.selection_button = MidiAssignment {
                    channel,
                    control,
                    control_type: ControlType::Button,
                };
            }

            // Move to next learning state
            self.state = ControllerState::LearningBackButton;
            self.ui.prompt_press_back_button()?;
        }

        Ok(())
    }

    /// Learn the back button assignment
    pub(super) fn learn_back_button(&mut self, event: driver::MidiEvent) -> Result<()> {
        if let driver::MidiEvent::ControlChange { channel, control, .. } = event {
            // Ignore if this is an already-learned control
            if let Some(config) = &self.base_control_config {
                if (config.main_knob.channel == channel && config.main_knob.control == control)
                    || (config.secondary_knob.channel == channel
                        && config.secondary_knob.control == control)
                    || (config.selection_button.channel == channel
                        && config.selection_button.control == control)
                {
                    debug!("Ignoring already-learned control event during back button learning");
                    return Ok(());
                }
            }

            info!("Learned back button: channel={}, cc={}", channel, control);

            if let Some(config) = &mut self.base_control_config {
                config.back_button = MidiAssignment {
                    channel,
                    control,
                    control_type: ControlType::Button,
                };
            }

            // Save configuration
            self.save_config()?;

            // Move to navigating state
            self.state = ControllerState::Navigating;
            self.ui.update_state("Navigating")?;
            info!("Learning complete, controller ready for navigation");
        }

        Ok(())
    }
}

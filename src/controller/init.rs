use super::{BaseControlConfig, ControlType, Controller, ControllerState, MidiAssignment};
use crate::controller::driver;
use anyhow::Result;
use log::{debug, info};

impl Controller {
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
            self.ui.signal_state_change("Navigating")?;
            info!("Learning complete, controller ready for navigation");
        }

        Ok(())
    }
}

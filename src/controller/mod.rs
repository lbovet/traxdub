pub mod init;
pub mod driver;

use crate::engine::Engine;
use crate::ui::UI;
use anyhow::Result;
use log::{debug, error, warn, trace};
use serde::{Deserialize, Serialize};
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

/// Knob direction
#[derive(Debug, Clone, PartialEq)]
pub enum KnobDirection {
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
    BrowsingMenu,
}

/// Main controller that processes MIDI events and coordinates engine and UI
pub struct Controller {
    ui: Arc<UI>,
    engine: Arc<Engine>,
    state: ControllerState,
    base_control_config: Option<BaseControlConfig>,
    config_path: PathBuf,
    force_init: bool,
    main_knob_accumulator: f32,
    secondary_knob_accumulator: f32,
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
            main_knob_accumulator: 0.0,
            secondary_knob_accumulator: 0.0,
        };
        
        controller.initialize()?;
        
        // Create initial system nodes in UI
        controller.ui.create_node("inputs".to_string(), "Inputs".to_string(), crate::ui::NodeType::System)?;
        controller.ui.create_node("outputs".to_string(), "Outputs".to_string(), crate::ui::NodeType::System)?;
        controller.ui.create_node("guitar".to_string(), "Guitar".to_string(), crate::ui::NodeType::Normal)?;
        controller.ui.create_node("mike".to_string(), "Mike".to_string(), crate::ui::NodeType::Normal)?;
        controller.ui.create_node("reverb".to_string(), "Reverb".to_string(), crate::ui::NodeType::Normal)?;  
        controller.ui.create_node("main".to_string(), "Main".to_string(), crate::ui::NodeType::Normal)?;        
        controller.ui.create_link("inputs".to_string(), "guitar".to_string())?;
        controller.ui.create_link("guitar".to_string(), "main".to_string())?;
        controller.ui.create_link("inputs".to_string(), "mike".to_string())?;
        controller.ui.create_link("mike".to_string(), "reverb".to_string())?;
        controller.ui.create_link("reverb".to_string(), "main".to_string())?;
        controller.ui.create_link("main".to_string(), "outputs".to_string())?;
        
        Ok(controller)
    }
    
    /// Process a MIDI event
    pub fn process_midi_event(&mut self, event: driver::MidiEvent) -> Result<()> {
        trace!("Processing MIDI event: {:?} in state {:?}", event, self.state);
        
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
            ControllerState::BrowsingMenu => {
                self.process_event_browsing_menu_state(event)?;
            }
            _ => {
                warn!("Received MIDI event in unexpected state: {:?}", self.state);
            }
        }
        
        Ok(())
    }
    
    /// Process events when in navigating state
    fn process_event_navigating_state(&mut self, event: driver::MidiEvent) -> Result<()> {
        const DELTA_THRESHOLD: f32 = 256.0;
        
        if let driver::MidiEvent::ControlChange { channel, control, value } = event {
            if let Some(config) = &self.base_control_config {
                // Check if it's the main knob
                if config.main_knob.channel == channel && config.main_knob.control == control {
                    if let Some(direction) = Self::process_knob_value(value, &mut self.main_knob_accumulator, DELTA_THRESHOLD) {
                        self.ui.navigate(NavigationLevel::Main, direction)?;
                    }
                }
                // Check if it's the secondary knob
                else if config.secondary_knob.channel == channel && config.secondary_knob.control == control {
                    if let Some(direction) = Self::process_knob_value(value, &mut self.secondary_knob_accumulator, DELTA_THRESHOLD) {
                        self.ui.navigate(NavigationLevel::Secondary, direction)?;
                    }
                }
                // Check if it's the selection button (button press: value > 0)
                else if config.selection_button.channel == channel && config.selection_button.control == control && value > 0 {
                    if let Some(element) = self.ui.select()? {
                        debug!("Selected element: {:?}", element);
                        
                        let stack_num = self.ui.menu_stack_size() + 1;
                        let menu = crate::ui::Menu {
                            id: format!("example_menu_{}", stack_num),
                            name: format!("Example Menu {}", stack_num),
                            options: vec![
                                crate::ui::MenuOption {
                                    id: "option1".to_string(),
                                    name: "Option 1".to_string(),
                                },
                                crate::ui::MenuOption {
                                    id: "option2".to_string(),
                                    name: "Option 2".to_string(),
                                },
                                crate::ui::MenuOption {
                                    id: "option3".to_string(),
                                    name: "Option 3".to_string(),
                                },
                            ],
                        };
                        self.ui.open_menu(menu)?;
                        self.state = ControllerState::BrowsingMenu;

                    }
                }
                // Check if it's the back button (button press: value > 0)
                else if config.back_button.channel == channel && config.back_button.control == control && value > 0 {
                    if let Err(e) = self.ui.close_menu() {
                        debug!("No menu to close: {}", e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Process events when in browsing menu state
    fn process_event_browsing_menu_state(&mut self, event: driver::MidiEvent) -> Result<()> {
        const DELTA_THRESHOLD: f32 = 256.0;
        
        if let driver::MidiEvent::ControlChange { channel, control, value } = event {
            if let Some(config) = &self.base_control_config {
                // Check if it's the main knob (navigate menu options)
                if config.main_knob.channel == channel && config.main_knob.control == control {
                    if let Some(direction) = Self::process_knob_value(value, &mut self.main_knob_accumulator, DELTA_THRESHOLD) {
                        self.ui.navigate(NavigationLevel::Main, direction)?;
                    }
                }
                // Secondary knob is not used in menu browsing
                // Check if it's the selection button (select menu option)
                else if config.selection_button.channel == channel && config.selection_button.control == control && value > 0 {
                    if let Some(element) = self.ui.select()? {
                        debug!("Selected element in menu: {:?}", element);
                        
                        // Check if first option was selected
                        if let crate::ui::Element::MenuOption(_, ref option_id) = element {
                            if option_id == "option1" {
                                // Open another menu
                                let stack_num = self.ui.menu_stack_size() + 1;
                                let submenu = crate::ui::Menu {
                                    id: format!("example_menu_{}", stack_num),
                                    name: format!("Example Menu {}", stack_num),
                                    options: vec![
                                        crate::ui::MenuOption {
                                            id: "suboption1".to_string(),
                                            name: "Sub Option 1".to_string(),
                                        },
                                        crate::ui::MenuOption {
                                            id: "suboption2".to_string(),
                                            name: "Sub Option 2".to_string(),
                                        },
                                        crate::ui::MenuOption {
                                            id: "suboption3".to_string(),
                                            name: "Sub Option 3".to_string(),
                                        },
                                    ],
                                };
                                self.ui.open_menu(submenu)?;
                                // Stay in BrowsingMenu state
                                return Ok(());
                            }
                        }

                        self.ui.close_all_menus()?;
                        self.state = ControllerState::Navigating;
                        // TODO: Handle menu option selection
                    }
                }
                // Check if it's the back button (close menu and return to Navigating state)
                else if config.back_button.channel == channel && config.back_button.control == control && value > 0 {
                    if self.ui.back()? {
                        self.state = ControllerState::Navigating;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Process knob value and return navigation direction if threshold is reached
    fn process_knob_value(value: u8, accumulator: &mut f32, threshold: f32) -> Option<KnobDirection> {
        let delta = if value >= 64 {
            (value - 64) as f32
        } else {
            -((64 - value) as f32)
        };
        
        *accumulator += delta;
        
        if accumulator.abs() >= threshold {
            let direction = if *accumulator > 0.0 {
                KnobDirection::Backward
            } else {
                KnobDirection::Forward
            };
            *accumulator = 0.0;
            Some(direction)
        } else {
            None
        }
    }
    
    /// Run loop with signal handling for graceful shutdown
    pub fn run_until_signal(&mut self, running: Arc<AtomicBool>) -> Result<()> {
        debug!("Controller running in state: {:?}", self.state);
        
        // Start MIDI receiver and get the event channel        
        let (driver, event_receiver) = driver::Driver::start()?;
        
        driver.connect_all_midi_inputs()?;
        
        // Process events from the receiver until signal
        while running.load(Ordering::SeqCst) {
            match event_receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
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
        
        debug!("Controller shutting down gracefully");
        drop(driver);
        debug!("MIDI receiver dropped");
        Ok(())
    }
}

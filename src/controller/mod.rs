pub mod init;
pub mod driver;
pub mod feature;

use crate::engine::Engine;
use crate::ui::UI;
use crate::controller::feature::Feature;
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
    driver: Arc<driver::Driver>,
    state: ControllerState,
    base_control_config: Option<BaseControlConfig>,
    config_path: PathBuf,
    force_init: bool,
    main_knob_accumulator: f32,
    secondary_knob_accumulator: f32,
    input_feature: Option<feature::InputFeature>,
    output_feature: Option<feature::OutputFeature>,
    plugin_feature: Option<feature::PluginFeature>,
    persistence_feature: Option<feature::PersistenceFeature>,
    current_feature: Option<*mut dyn Feature>,
    /// The UI element that was selected when opening the current feature
    current_element: Option<crate::ui::Element>,
}

// Mark Controller as Send - the raw pointer is only used within the controller's methods
// and never sent across threads
unsafe impl Send for Controller {}

impl Controller {
    /// Create a new controller instance
    pub fn new(ui: Arc<UI>, engine: Arc<Engine>, force_init: bool, new_session: bool) -> Result<Self> {
        let config_path = Self::get_config_path();
        
        // Create JACK driver
        let driver = Arc::new(driver::Driver::new()?);
        
        let mut controller = Self {
            ui: ui.clone(),
            engine: engine.clone(),
            driver,
            state: ControllerState::Initializing,
            base_control_config: None,
            config_path,
            force_init,
            main_knob_accumulator: 0.0,
            secondary_knob_accumulator: 0.0,
            input_feature: None,
            output_feature: None,
            plugin_feature: None,
            persistence_feature: None,
            current_feature: None,
            current_element: None,
        };
        
        controller.initialize()?;
        
        // Create initial context nodes in UI
        controller.ui.create_node("inputs".to_string(), "Inputs".to_string(), crate::ui::NodeType::Context)?;
        controller.ui.create_node("outputs".to_string(), "Outputs".to_string(), crate::ui::NodeType::Context)?;
        // Context -> Context => Virtual
        controller.ui.create_link("inputs".to_string(), "outputs".to_string(), crate::ui::LinkType::Virtual)?;
        
        // Initialize persistence feature with auto-load flag
        let auto_load = !new_session; // auto_load is opposite of new_session
        controller.persistence_feature = Some(feature::new_persistence_feature(
            Arc::clone(&controller.driver),
            engine.clone(),
            ui.clone(),
            auto_load,
        ));
        
        // Initialize input feature with driver and engine
        controller.input_feature = Some(feature::new_input_feature(
            Arc::clone(&controller.driver),
            Arc::clone(&engine),
            Arc::clone(&ui),
        ));
        
        // Initialize output feature with driver and engine
        controller.output_feature = Some(feature::new_output_feature(
            Arc::clone(&controller.driver),
            Arc::clone(&engine),
            Arc::clone(&ui),
        ));
        
        // Initialize plugin feature
        controller.plugin_feature = Some(feature::new_plugin_feature(
            Arc::clone(&engine),
            Arc::clone(&ui),
        ));
        
        Ok(controller)
    }
    
    /// Get a reference to the current active feature
    fn current_feature(&self) -> Option<&dyn Feature> {
        self.current_feature.map(|ptr| unsafe { &*ptr })
    }
    
    /// Get a mutable reference to the current active feature
    fn current_feature_mut(&mut self) -> Option<&mut dyn Feature> {
        self.current_feature.map(|ptr| unsafe { &mut *ptr })
    }
    
    /// Process a MIDI event
    pub fn process_midi_event(&mut self, event: driver::MidiEvent) -> Result<()> {
        trace!("Processing event: {:?} in state {:?}", event, self.state);
        
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
                warn!("Received event in unexpected state: {:?}", self.state);
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
                        
                        // Store the selected element for use by features
                        self.current_element = Some(element.clone());
                                               
                        if let crate::ui::Element::Link(ref from_id, ref to_id, _) = element {
                            // Build menu options based on link endpoints
                            let mut options = Vec::new();
                            
                            if from_id == "inputs" {
                                options.push(crate::ui::MenuOption {
                                    id: "add_input".to_string(),
                                    name: "Add Input...".to_string(),
                                });
                            }
                            
                            if to_id == "outputs" {
                                options.push(crate::ui::MenuOption {
                                    id: "add_output".to_string(),
                                    name: "Add Output...".to_string(),
                                });
                            }
                            
                            // Add plugin option if upstream is not "inputs"
                            if from_id != "inputs" {
                                options.push(crate::ui::MenuOption {
                                    id: "add_plugin".to_string(),
                                    name: "Add Plugin...".to_string(),
                                });
                            }
                            
                            // Add File option (always available)
                            options.push(crate::ui::MenuOption {
                                id: "file".to_string(),
                                name: "File...".to_string(),
                            });
                            
                            // Open menu if we have at least one option
                            if !options.is_empty() {
                                let menu = crate::ui::Menu {
                                    id: format!("link_{}_{}", from_id, to_id),
                                    name: format!("{} → {}", from_id, to_id),
                                    options,
                                };
                                self.ui.open_menu(menu)?;
                                self.state = ControllerState::BrowsingMenu;
                            }
                        } else {
                            // For node elements, only show File menu
                            let menu = crate::ui::Menu {
                                id: "node_menu".to_string(),
                                name: "Node".to_string(),
                                options: vec![
                                    crate::ui::MenuOption {
                                        id: "file".to_string(),
                                        name: "File...".to_string(),
                                    },
                                ],
                            };
                            self.ui.open_menu(menu)?;
                            self.state = ControllerState::BrowsingMenu;
                        }
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
                        
                        if let crate::ui::Element::MenuOption(_, ref option_id) = element {
                            // Handle special link menu options
                            if option_id == "add_input" {
                                self.current_feature = self.input_feature.as_mut().map(|f| f as *mut dyn Feature);
                                // Open the input feature menu on top of the link menu
                                if let Some(feature) = self.current_feature() {
                                    let menu = feature.get_menu();
                                    self.ui.open_menu(menu)?;
                                }
                                return Ok(());
                            } else if option_id == "add_output" {
                                self.current_feature = self.output_feature.as_mut().map(|f| f as *mut dyn Feature);
                                // Open the output feature menu on top of the link menu
                                if let Some(feature) = self.current_feature() {
                                    let menu = feature.get_menu();
                                    self.ui.open_menu(menu)?;
                                }
                                return Ok(());
                            } else if option_id == "add_plugin" {
                                self.current_feature = self.plugin_feature.as_mut().map(|f| f as *mut dyn Feature);
                                // Open the plugin feature menu on top of the link menu
                                if let Some(feature) = self.current_feature() {
                                    let menu = feature.get_menu();
                                    self.ui.open_menu(menu)?;
                                }
                                return Ok(());
                            } else if option_id == "file" {
                                self.current_feature = self.persistence_feature.as_mut().map(|f| f as *mut dyn Feature);
                                // Open the file feature menu on top of the current menu
                                if let Some(feature) = self.current_feature() {
                                    let menu = feature.get_menu();
                                    self.ui.open_menu(menu)?;
                                }
                                return Ok(());
                            }
                            
                            // Handle menu option through the current active feature
                            let current_elem = self.current_element.clone();
                            let next_state = if let Some(feature) = self.current_feature_mut() {
                                feature.handle_menu_option(Some(option_id), current_elem.as_ref())?
                            } else {
                                ControllerState::Navigating
                            };
                            
                            match next_state {
                                ControllerState::BrowsingMenu => {
                                    // Open the next menu from the current feature
                                    if let Some(feature) = self.current_feature() {
                                        let menu = feature.get_menu();
                                        self.ui.open_menu(menu)?;
                                    }
                                }
                                ControllerState::Navigating => {
                                    // Close all menus and return to navigating
                                    self.ui.close_all_menus()?;
                                    self.current_feature = None;
                                    self.current_element = None;
                                    self.state = ControllerState::Navigating;
                                }
                                _ => {
                                    // For other states, just transition
                                    self.state = next_state;
                                }
                            }
                        }
                    }
                }
                // Check if it's the back button (close menu and return to Navigating state)
                else if config.back_button.channel == channel && config.back_button.control == control && value > 0 {
                    if self.ui.back()? {
                        // If no more menus, return to Navigating
                        if self.ui.menu_stack_size() == 0 {
                            self.current_feature = None;
                            self.state = ControllerState::Navigating;
                        }
                    } else {
                        let current_elem = self.current_element.clone();
                        if let Some(feature) = self.current_feature_mut() {
                            feature.handle_menu_option(None, current_elem.as_ref())?;
                        }
                        self.current_element = None;
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
        let event_receiver = self.driver.start()?;
        
        self.driver.connect_all_midi_inputs()?;
        
        // Note: All features are initialized in Controller::new()
        
        // Process events from the receiver until signal
        while running.load(Ordering::SeqCst) {
            match event_receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    if let Err(e) = self.process_midi_event(event) {
                        warn!("Error processing event: {}", e);
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
        debug!("MIDI receiver stopped");

        // Explicitly drop engine to ensure clean shutdown
        debug!("Dropping engine...");
        self.engine.close();

        Ok(())
    }
}

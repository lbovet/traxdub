use anyhow::Result;
use log::{debug, warn};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::controller::driver::{Driver, PortType};
use crate::controller::{ControllerState, feature::Feature};
use crate::engine::Engine;
use crate::ui::{Menu, MenuOption};

/// Menu state for the input feature
#[derive(Debug, Clone, PartialEq)]
enum InputMenuState {
    MainMenu,
    PortTypeSelection,
    SourceList(PortType), // Contains the selected port type
    PortList(PortType, String), // Contains port type and source name
}

/// Input feature for managing audio/MIDI inputs
pub struct InputFeature {
    driver: Arc<Driver>,
    engine: Arc<Engine>,
    menu_state: InputMenuState,
}

impl InputFeature {
    /// Create a new input feature
    pub fn new(driver: Arc<Driver>, engine: Arc<Engine>) -> Self {
        Self {
            driver,
            engine,
            menu_state: InputMenuState::MainMenu,
        }
    }

    /// Sanitize port name for engine use
    /// Converts to lowercase and replaces sequences of special chars with single underscore
    fn sanitize_port_name(name: &str) -> String {
        let mut result = String::new();
        let mut last_was_special = false;
        
        for c in name.chars() {
            if c.is_alphanumeric() {
                result.push(c.to_lowercase().next().unwrap());
                last_was_special = false;
            } else {
                if !last_was_special && !result.is_empty() {
                    result.push('_');
                }
                last_was_special = true;
            }
        }
        
        // Remove trailing underscore if any
        if result.ends_with('_') {
            result.pop();
        }
        
        result
    }

    /// Get the main menu for input feature
    fn get_main_menu(&self) -> Menu {
        Menu {
            id: "input_main".to_string(),
            name: "Input".to_string(),
            options: vec![
                MenuOption {
                    id: "add".to_string(),
                    name: "Add...".to_string(),
                },
            ],
        }
    }

    /// Get the port type selection menu
    fn get_port_type_menu(&self) -> Menu {
        Menu {
            id: "input_port_type".to_string(),
            name: "Select Port Type".to_string(),
            options: vec![
                MenuOption {
                    id: "type_audio".to_string(),
                    name: "Audio".to_string(),
                },
                MenuOption {
                    id: "type_midi".to_string(),
                    name: "MIDI".to_string(),
                },
            ],
        }
    }

    /// Get the source selection menu
    fn get_source_menu(&self, port_type: PortType) -> Result<Menu> {
        let sources = self.driver.get_sources(port_type)?;
        
        let options: Vec<MenuOption> = sources.iter().map(|source| {
            MenuOption {
                id: format!("source_{}", source.name),
                name: source.name.clone(),
            }
        }).collect();

        Ok(Menu {
            id: "input_sources".to_string(),
            name: "Select Input Source".to_string(),
            options,
        })
    }

    /// Get the port selection menu for a specific source
    fn get_port_menu(&self, port_type: PortType, source_name: &str) -> Result<Menu> {
        let sources = self.driver.get_sources(port_type)?;
        
        let source = sources.iter()
            .find(|s| s.name == source_name)
            .ok_or_else(|| anyhow::anyhow!("Source not found: {}", source_name))?;

        let options: Vec<MenuOption> = source.ports.iter().map(|port| {
            MenuOption {
                id: format!("port_{}", port.name),
                name: port.short_name.clone(),
            }
        }).collect();

        Ok(Menu {
            id: format!("input_ports_{}", source_name),
            name: format!("Ports: {}", source_name),
            options,
        })
    }
}

impl Feature for InputFeature {
    fn get_menu(&self) -> Menu {
        match &self.menu_state {
            InputMenuState::MainMenu => self.get_main_menu(),
            InputMenuState::PortTypeSelection => self.get_port_type_menu(),
            InputMenuState::SourceList(port_type) => {
                self.get_source_menu(*port_type).unwrap_or_else(|e| {
                    debug!("Error getting source menu: {}", e);
                    self.get_main_menu()
                })
            }
            InputMenuState::PortList(port_type, source_name) => {
                self.get_port_menu(*port_type, source_name).unwrap_or_else(|e| {
                    debug!("Error getting port menu for {}: {}", source_name, e);
                    self.get_main_menu()
                })
            }
        }
    }

    fn handle_menu_option(&mut self, option_id: &str) -> Result<ControllerState> {
        debug!("Input feature handling option: {}", option_id);

        match &self.menu_state {
            InputMenuState::MainMenu => {
                if option_id == "add" {
                    // Transition to port type selection
                    self.menu_state = InputMenuState::PortTypeSelection;
                    Ok(ControllerState::BrowsingMenu)
                } else {
                    Ok(ControllerState::Navigating)
                }
            }
            InputMenuState::PortTypeSelection => {
                let port_type = match option_id {
                    "type_audio" => PortType::Audio,
                    "type_midi" => PortType::Midi,
                    _ => {
                        self.menu_state = InputMenuState::MainMenu;
                        return Ok(ControllerState::Navigating);
                    }
                };
                // Transition to source list with selected port type
                self.menu_state = InputMenuState::SourceList(port_type);
                Ok(ControllerState::BrowsingMenu)
            }
            InputMenuState::SourceList(port_type) => {
                if let Some(source_name) = option_id.strip_prefix("source_") {
                    // Transition to port list for this source
                    self.menu_state = InputMenuState::PortList(*port_type, source_name.to_string());
                    Ok(ControllerState::BrowsingMenu)
                } else {
                    self.menu_state = InputMenuState::MainMenu;
                    Ok(ControllerState::Navigating)
                }
            }
            InputMenuState::PortList(port_type, source_name) => {
                if let Some(port_name) = option_id.strip_prefix("port_") {
                    debug!("Selected {} port: {} from source: {}", 
                           match port_type {
                               PortType::Audio => "Audio",
                               PortType::Midi => "MIDI",
                               PortType::All => "All",
                           },
                           port_name, source_name);
                    
                    // Sanitize the port name
                    let sanitized_name = Self::sanitize_port_name(port_name);
                    debug!("Sanitized port name: {}", sanitized_name);
                    
                    // Convert PortType from driver to engine
                    let engine_port_type = match port_type {
                        PortType::Audio => crate::engine::PortType::Audio,
                        PortType::Midi => crate::engine::PortType::Midi,
                        PortType::All => crate::engine::PortType::Audio, // Default to Audio if All
                    };
                    
                    // Create input port in engine
                    self.engine.create_input_port(&sanitized_name, engine_port_type)?;
                    
                    // Connect the JACK port to the engine port
                    // Retry connection as the engine port is created asynchronously
                    let source_port = crate::controller::driver::Port {
                        name: port_name.to_string(),
                        short_name: port_name.split(':').last().unwrap_or(port_name).to_string(),
                    };
                    let destination_port = crate::controller::driver::Port {
                        name: format!("TraxDub Engine:{}", sanitized_name),
                        short_name: sanitized_name.clone(),
                    };
                    
                    let max_duration = Duration::from_millis(1000);
                    let retry_interval = Duration::from_millis(50);
                    let start_time = std::time::Instant::now();
                    let mut connected = false;
                    
                    while start_time.elapsed() < max_duration {
                        match self.driver.connect_ports(&source_port, &destination_port) {
                            Ok(_) => {
                                connected = true;
                                break;
                            }
                            Err(e) => {
                                warn!("Connection attempt failed ({}ms elapsed): {}", 
                                      start_time.elapsed().as_millis(), e);
                                thread::sleep(retry_interval);
                            }
                        }
                    }
                    
                    if !connected {
                        return Err(anyhow::anyhow!(
                            "Failed to connect ports after {} retries over {}ms",
                            max_duration.as_millis() / retry_interval.as_millis(),
                            max_duration.as_millis()
                        ));
                    }
                    
                    debug!("Successfully created and connected input port: {}", sanitized_name);
                    
                    self.menu_state = InputMenuState::MainMenu;
                    Ok(ControllerState::Navigating)
                } else {
                    self.menu_state = InputMenuState::MainMenu;
                    Ok(ControllerState::Navigating)
                }
            }
        }
    }

    fn reset(&mut self) {
        self.menu_state = InputMenuState::MainMenu;
    }
}

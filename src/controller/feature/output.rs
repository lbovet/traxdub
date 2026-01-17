use anyhow::Result;
use log::{debug, warn};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::controller::driver::{Driver, PortType};
use crate::controller::{ControllerState, feature::Feature};
use crate::engine::Engine;
use crate::ui::{Menu, MenuOption};

/// Menu state for the output feature
#[derive(Debug, Clone, PartialEq)]
enum OutputMenuState {
    MainMenu,
    PortTypeSelection,
    DestinationList(PortType), // Contains the selected port type
    PortList(PortType, String), // Contains port type and destination name
}

/// Output feature for managing audio/MIDI outputs
pub struct OutputFeature {
    driver: Arc<Driver>,
    engine: Arc<Engine>,
    menu_state: OutputMenuState,
}

impl OutputFeature {
    /// Create a new output feature
    pub fn new(driver: Arc<Driver>, engine: Arc<Engine>) -> Self {
        Self {
            driver,
            engine,
            menu_state: OutputMenuState::MainMenu,
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

    /// Get the main menu for output feature
    fn get_main_menu(&self) -> Menu {
        Menu {
            id: "output_main".to_string(),
            name: "Output".to_string(),
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
            id: "output_port_type".to_string(),
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

    /// Get the destination selection menu
    fn get_destination_menu(&self, port_type: PortType) -> Result<Menu> {
        let sinks = self.driver.get_sinks(port_type)?;
        
        let options: Vec<MenuOption> = sinks.iter().map(|sink| {
            MenuOption {
                id: format!("destination_{}", sink.name),
                name: sink.name.clone(),
            }
        }).collect();

        Ok(Menu {
            id: "output_destinations".to_string(),
            name: "Select Output Destination".to_string(),
            options,
        })
    }

    /// Get the port selection menu for a specific destination
    fn get_port_menu(&self, port_type: PortType, destination_name: &str) -> Result<Menu> {
        let sinks = self.driver.get_sinks(port_type)?;
        
        let sink = sinks.iter()
            .find(|s| s.name == destination_name)
            .ok_or_else(|| anyhow::anyhow!("Destination not found: {}", destination_name))?;

        let options: Vec<MenuOption> = sink.ports.iter().map(|port| {
            MenuOption {
                id: format!("port_{}", port.name),
                name: port.short_name.clone(),
            }
        }).collect();

        Ok(Menu {
            id: format!("output_ports_{}", destination_name),
            name: format!("Ports: {}", destination_name),
            options,
        })
    }
}

impl Feature for OutputFeature {
    fn get_menu(&self) -> Menu {
        match &self.menu_state {
            OutputMenuState::MainMenu => self.get_main_menu(),
            OutputMenuState::PortTypeSelection => self.get_port_type_menu(),
            OutputMenuState::DestinationList(port_type) => {
                self.get_destination_menu(*port_type).unwrap_or_else(|e| {
                    debug!("Error getting destination menu: {}", e);
                    self.get_main_menu()
                })
            }
            OutputMenuState::PortList(port_type, destination_name) => {
                self.get_port_menu(*port_type, destination_name).unwrap_or_else(|e| {
                    debug!("Error getting port menu for {}: {}", destination_name, e);
                    self.get_main_menu()
                })
            }
        }
    }

    fn handle_menu_option(&mut self, option_id: Option<&str>) -> Result<ControllerState> {
        // Handle menu closure - revert to previous menu state
        let Some(option_id) = option_id else {
            debug!("Output feature: menu closed, reverting to previous state");
            match &self.menu_state {
                OutputMenuState::MainMenu => {
                    // Already at main menu, exit to navigating
                    return Ok(ControllerState::Navigating);
                }
                OutputMenuState::PortTypeSelection => {
                    self.menu_state = OutputMenuState::MainMenu;
                    return Ok(ControllerState::BrowsingMenu);
                }
                OutputMenuState::DestinationList(_) => {
                    self.menu_state = OutputMenuState::PortTypeSelection;
                    return Ok(ControllerState::BrowsingMenu);
                }
                OutputMenuState::PortList(port_type, _) => {
                    self.menu_state = OutputMenuState::DestinationList(*port_type);
                    return Ok(ControllerState::BrowsingMenu);
                }
            }
        };

        debug!("Output feature handling option: {}", option_id);

        match &self.menu_state {
            OutputMenuState::MainMenu => {
                if option_id == "add" {
                    // Transition to port type selection
                    self.menu_state = OutputMenuState::PortTypeSelection;
                    Ok(ControllerState::BrowsingMenu)
                } else {
                    Ok(ControllerState::Navigating)
                }
            }
            OutputMenuState::PortTypeSelection => {
                let port_type = match option_id {
                    "type_audio" => PortType::Audio,
                    "type_midi" => PortType::Midi,
                    _ => {
                        self.menu_state = OutputMenuState::MainMenu;
                        return Ok(ControllerState::Navigating);
                    }
                };
                // Transition to destination list with selected port type
                self.menu_state = OutputMenuState::DestinationList(port_type);
                Ok(ControllerState::BrowsingMenu)
            }
            OutputMenuState::DestinationList(port_type) => {
                if let Some(destination_name) = option_id.strip_prefix("destination_") {
                    // Transition to port list for this destination
                    self.menu_state = OutputMenuState::PortList(*port_type, destination_name.to_string());
                    Ok(ControllerState::BrowsingMenu)
                } else {
                    self.menu_state = OutputMenuState::MainMenu;
                    Ok(ControllerState::Navigating)
                }
            }
            OutputMenuState::PortList(port_type, destination_name) => {
                if let Some(port_name) = option_id.strip_prefix("port_") {
                    debug!("Selected {} port: {} from destination: {}", 
                           match port_type {
                               PortType::Audio => "Audio",
                               PortType::Midi => "MIDI",
                               PortType::All => "All",
                           },
                           port_name, destination_name);
                    
                    // Sanitize the port name
                    let sanitized_name = Self::sanitize_port_name(port_name);
                    debug!("Sanitized port name: {}", sanitized_name);
                    
                    // Convert PortType from driver to engine
                    let engine_port_type = match port_type {
                        PortType::Audio => crate::engine::PortType::Audio,
                        PortType::Midi => crate::engine::PortType::Midi,
                        PortType::All => crate::engine::PortType::Audio, // Default to Audio if All
                    };
                    
                    // Create output port in engine
                    self.engine.create_output_port(&sanitized_name, engine_port_type)?;
                    
                    // Connect the engine port to the JACK port
                    // Retry connection as the engine port is created asynchronously
                    let source_port = crate::controller::driver::Port {
                        name: format!("TraxDub Engine:{}", sanitized_name),
                        short_name: sanitized_name.clone(),
                    };
                    let destination_port = crate::controller::driver::Port {
                        name: port_name.to_string(),
                        short_name: port_name.split(':').last().unwrap_or(port_name).to_string(),
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
                    
                    debug!("Successfully created and connected output port: {}", sanitized_name);
                    
                    self.menu_state = OutputMenuState::MainMenu;
                    Ok(ControllerState::Navigating)
                } else {
                    self.menu_state = OutputMenuState::MainMenu;
                    Ok(ControllerState::Navigating)
                }
            }
        }
    }
}

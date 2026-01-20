use anyhow::Result;
use log::{debug, warn};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::controller::driver::{Driver, PortType};
use crate::controller::{ControllerState, feature::Feature};
use crate::engine::Engine;
use crate::ui::{Menu, MenuOption, UI, NodeType};

/// Direction of the system feature (input or output)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemDirection {
    Input,
    Output,
}

/// Menu state for the system feature
#[derive(Debug, Clone, PartialEq)]
enum SystemMenuState {
    PortTypeSelection,
    EndpointList(PortType), // Contains the selected port type (source/sink list)
    PortList(PortType, String), // Contains port type and endpoint name
}

/// System feature for managing audio/MIDI inputs or outputs
pub struct SystemFeature {
    driver: Arc<Driver>,
    engine: Arc<Engine>,
    ui: Arc<UI>,
    menu_state: SystemMenuState,
    direction: SystemDirection,
}

impl SystemFeature {
    /// Create a new system feature with specified direction
    pub fn new(driver: Arc<Driver>, engine: Arc<Engine>, ui: Arc<UI>, direction: SystemDirection) -> Self {
        Self {
            driver,
            engine,
            ui,
            menu_state: SystemMenuState::PortTypeSelection,
            direction
        }
    }

    /// Get the direction name (lowercase)
    fn direction_name(&self) -> &str {
        match self.direction {
            SystemDirection::Input => "input",
            SystemDirection::Output => "output",
        }
    }

    /// Get the direction name (capitalized)
    fn direction_name_cap(&self) -> &str {
        match self.direction {
            SystemDirection::Input => "Input",
            SystemDirection::Output => "Output",
        }
    }

    /// Get the endpoint type name (source/destination)
    fn endpoint_type_name(&self) -> &str {
        match self.direction {
            SystemDirection::Input => "source",
            SystemDirection::Output => "destination",
        }
    }

    /// Get the endpoint type name (capitalized)
    fn endpoint_type_name_cap(&self) -> &str {
        match self.direction {
            SystemDirection::Input => "Source",
            SystemDirection::Output => "Destination",
        }
    }

    /// Get the port type selection menu
    fn get_port_type_menu(&self) -> Menu {
        Menu {
            id: format!("{}_port_type", self.direction_name()),
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

    /// Get the endpoint selection menu (sources for input, sinks for output)
    fn get_endpoint_menu(&self, port_type: PortType) -> Result<Menu> {
        let options: Vec<MenuOption> = match self.direction {
            SystemDirection::Input => {
                let endpoints = self.driver.get_sources(port_type)?;
                endpoints.iter().map(|endpoint| {
                    MenuOption {
                        id: format!("{}_{}", self.endpoint_type_name(), endpoint.name),
                        name: endpoint.name.clone(),
                    }
                }).collect()
            }
            SystemDirection::Output => {
                let endpoints = self.driver.get_sinks(port_type)?;
                endpoints.iter().map(|endpoint| {
                    MenuOption {
                        id: format!("{}_{}", self.endpoint_type_name(), endpoint.name),
                        name: endpoint.name.clone(),
                    }
                }).collect()
            }
        };

        Ok(Menu {
            id: format!("{}_{}", self.direction_name(), match self.direction {
                SystemDirection::Input => "sources",
                SystemDirection::Output => "destinations",
            }),
            name: format!("Select {} {}", self.direction_name_cap(), self.endpoint_type_name_cap()),
            options,
        })
    }

    /// Get the port selection menu for a specific endpoint
    fn get_port_menu(&self, port_type: PortType, endpoint_name: &str) -> Result<Menu> {
        let options: Vec<MenuOption> = match self.direction {
            SystemDirection::Input => {
                let endpoints = self.driver.get_sources(port_type)?;
                let endpoint = endpoints.iter()
                    .find(|e| e.name == endpoint_name)
                    .ok_or_else(|| anyhow::anyhow!("{} not found: {}", 
                        self.endpoint_type_name_cap(), endpoint_name))?;

                endpoint.ports.iter().map(|port| {
                    MenuOption {
                        id: format!("port_{}", port.name),
                        name: port.short_name.clone(),
                    }
                }).collect()
            }
            SystemDirection::Output => {
                let endpoints = self.driver.get_sinks(port_type)?;
                let endpoint = endpoints.iter()
                    .find(|e| e.name == endpoint_name)
                    .ok_or_else(|| anyhow::anyhow!("{} not found: {}", 
                        self.endpoint_type_name_cap(), endpoint_name))?;

                endpoint.ports.iter().map(|port| {
                    MenuOption {
                        id: format!("port_{}", port.name),
                        name: port.short_name.clone(),
                    }
                }).collect()
            }
        };

        Ok(Menu {
            id: format!("{}_ports_{}", self.direction_name(), endpoint_name),
            name: format!("Ports: {}", endpoint_name),
            options,
        })
    }
}

impl Feature for SystemFeature {
    fn get_menu(&self) -> Menu {
        match &self.menu_state {
            SystemMenuState::PortTypeSelection => self.get_port_type_menu(),
            SystemMenuState::EndpointList(port_type) => {
                self.get_endpoint_menu(*port_type).unwrap_or_else(|e| {
                    debug!("Error getting endpoint menu: {}", e);
                    self.get_port_type_menu()
                })
            }
            SystemMenuState::PortList(port_type, endpoint_name) => {
                self.get_port_menu(*port_type, endpoint_name).unwrap_or_else(|e| {
                    debug!("Error getting port menu for {}: {}", endpoint_name, e);
                    self.get_port_type_menu()
                })
            }
        }
    }

    fn handle_menu_option(&mut self, option_id: Option<&str>, element: Option<&crate::ui::Element>) -> Result<ControllerState> {
        debug!("{} feature handle_menu_option called with element: {:?}", self.direction_name_cap(), element);
        
        // Handle menu closure - revert to previous menu state
        let Some(option_id) = option_id else {
            debug!("{} feature: menu closed, reverting to previous state", self.direction_name_cap());
            match &self.menu_state {
                SystemMenuState::PortTypeSelection => {
                    // At first menu, exit to navigating
                    return Ok(ControllerState::Navigating);
                }
                SystemMenuState::EndpointList(_) => {
                    self.menu_state = SystemMenuState::PortTypeSelection;
                    return Ok(ControllerState::BrowsingMenu);
                }
                SystemMenuState::PortList(port_type, _) => {
                    self.menu_state = SystemMenuState::EndpointList(*port_type);
                    return Ok(ControllerState::BrowsingMenu);
                }
            }
        };

        debug!("{} feature handling option: {}", self.direction_name_cap(), option_id);

        match &self.menu_state {
            SystemMenuState::PortTypeSelection => {
                let port_type = match option_id {
                    "type_audio" => PortType::Audio,
                    "type_midi" => PortType::Midi,
                    _ => {
                        return Ok(ControllerState::Navigating);
                    }
                };
                // Transition to endpoint list with selected port type
                self.menu_state = SystemMenuState::EndpointList(port_type);
                Ok(ControllerState::BrowsingMenu)
            }
            SystemMenuState::EndpointList(port_type) => {
                if let Some(endpoint_name) = option_id.strip_prefix(&format!("{}_", self.endpoint_type_name())) {
                    // Transition to port list for this endpoint
                    self.menu_state = SystemMenuState::PortList(*port_type, endpoint_name.to_string());
                    Ok(ControllerState::BrowsingMenu)
                } else {
                    self.menu_state = SystemMenuState::PortTypeSelection;
                    Ok(ControllerState::Navigating)
                }
            }
            SystemMenuState::PortList(port_type, endpoint_name) => {
                if let Some(port_name) = option_id.strip_prefix("port_") {
                    debug!("Selected {} port: {} from {}: {}", 
                           match port_type {
                               PortType::Audio => "Audio",
                               PortType::Midi => "MIDI",
                               PortType::All => "All",
                           },
                           port_name, self.endpoint_type_name(), endpoint_name);
                    
                    // Sanitize the port name
                    let sanitized_name = Driver::sanitize_port_name(port_name);
                    debug!("Sanitized port name: {}", sanitized_name);
                    
                    // Convert PortType from driver to engine
                    let engine_port_type = match port_type {
                        PortType::Audio => crate::engine::PortType::Audio,
                        PortType::Midi => crate::engine::PortType::Midi,
                        PortType::All => crate::engine::PortType::Audio, // Default to Audio if All
                    };
                    
                    // Create port in engine (input or output based on direction)
                    let port_path = match self.direction {
                        SystemDirection::Input => {
                            self.engine.create_input_port(&sanitized_name, engine_port_type)?
                        }
                        SystemDirection::Output => {
                            self.engine.create_output_port(&sanitized_name, engine_port_type)?
                        }
                    };
                    
                    debug!("Created {} port at path: {}", self.direction_name(), port_path);
                    
                    // Set up JACK ports for connection based on direction
                    let (source_port, destination_port) = match self.direction {
                        SystemDirection::Input => {
                            // Input: connect FROM external source TO engine
                            (
                                crate::controller::driver::Port {
                                    name: port_name.to_string(),
                                    short_name: port_name.split(':').last().unwrap_or(port_name).to_string(),
                                },
                                crate::controller::driver::Port {
                                    name: format!("TraxDub Engine:{}", sanitized_name),
                                    short_name: sanitized_name.clone(),
                                }
                            )
                        }
                        SystemDirection::Output => {
                            // Output: connect FROM engine TO external destination
                            (
                                crate::controller::driver::Port {
                                    name: format!("TraxDub Engine:{}", sanitized_name),
                                    short_name: sanitized_name.clone(),
                                },
                                crate::controller::driver::Port {
                                    name: port_name.to_string(),
                                    short_name: port_name.split(':').last().unwrap_or(port_name).to_string(),
                                }
                            )
                        }
                    };
                    
                    // Retry connection as the engine port is created asynchronously
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
                    
                    debug!("Successfully created and connected {} port: {}", 
                           self.direction_name(), port_path);
                    
                    let port_type = match self.direction {
                        SystemDirection::Input => NodeType::PortIn,
                        SystemDirection::Output => NodeType::PortOut,
                    };

                    // Insert port node in UI, using link from/to if available
                    // If the selected link type is PortIn or PortOut, use "inputs" and "outputs"
                    // to avoid chaining PortIn nodes or PortOut nodes
                    let (link_from, link_to) = if let Some(crate::ui::Element::Link(from, to, link_type)) = &element {
                        // If link type is PortIn or PortOut, use inputs/outputs to avoid chaining
                        if matches!(link_type, crate::ui::LinkType::PortIn | crate::ui::LinkType::PortOut) {
                            ("inputs".to_string(), "outputs".to_string())
                        } else {
                            (from.clone(), to.clone())
                        }
                    } else {
                        ("inputs".to_string(), "outputs".to_string())
                    };

                    self.ui.insert_node(
                        port_path.clone(),
                        port_name.split(':').last().unwrap_or(port_name).to_string(),
                        port_type,
                        link_from.clone(),
                        link_to.clone(),
                    )?;
                    
                    // Create connections in the engine (skip "inputs" and "outputs" context nodes)
                    if link_from != "inputs" {
                        debug!("Creating engine connection: {} -> {}", link_from, port_path);
                        self.engine.connect(&link_from, &port_path)?;
                    }
                    if link_to != "outputs" {
                        debug!("Creating engine connection: {} -> {}", port_path, link_to);
                        self.engine.connect(&port_path, &link_to)?;
                    }
                    
                    self.menu_state = SystemMenuState::PortTypeSelection;
                    Ok(ControllerState::Navigating)
                } else {
                    self.menu_state = SystemMenuState::PortTypeSelection;
                    Ok(ControllerState::Navigating)
                }
            }
        }
    }
}

/// Input feature for managing audio/MIDI inputs
/// This is a type alias for SystemFeature configured for input direction
pub type InputFeature = SystemFeature;

/// Helper to create a new input feature
pub fn new_input_feature(driver: Arc<Driver>, engine: Arc<Engine>, ui: Arc<UI>) -> InputFeature {
    SystemFeature::new(driver, engine, ui, SystemDirection::Input)
}

/// Output feature for managing audio/MIDI outputs
/// This is a type alias for SystemFeature configured for output direction
pub type OutputFeature = SystemFeature;

/// Helper to create a new output feature
pub fn new_output_feature(driver: Arc<Driver>, engine: Arc<Engine>, ui: Arc<UI>) -> OutputFeature {
    SystemFeature::new(driver, engine, ui, SystemDirection::Output)
}

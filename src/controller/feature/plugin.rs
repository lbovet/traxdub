use anyhow::Result;
use log::{debug, warn};
use std::sync::Arc;

use crate::controller::{ControllerState, feature::Feature};
use crate::engine::{Engine, PortDirection};
use crate::ui::{Menu, MenuOption, UI, NodeType};

/// Menu state for the plugin feature
#[derive(Debug, Clone, PartialEq)]
enum PluginMenuState {
    PluginSelection,
}

/// Plugin feature for adding plugin blocks to the signal chain
pub struct PluginFeature {
    engine: Arc<Engine>,
    ui: Arc<UI>,
    menu_state: PluginMenuState,
    ui_element: Option<crate::ui::Element>,
}

impl PluginFeature {
    /// Create a new plugin feature
    pub fn new(engine: Arc<Engine>, ui: Arc<UI>) -> Self {
        Self {
            engine,
            ui,
            menu_state: PluginMenuState::PluginSelection,
            ui_element: None,
        }
    }
    
    /// Get the plugin selection menu
    fn get_plugin_selection_menu(&self) -> Menu {
        let plugins = self.engine.list_plugins();
        
        let options: Vec<MenuOption> = plugins.iter()
            .map(|plugin| MenuOption {
                id: plugin.id.clone(),
                name: plugin.name.clone(),
            })
            .collect();
        
        Menu {
            id: "plugin_selection".to_string(),
            name: "Select Plugin".to_string(),
            options,
        }
    }
}

impl Feature for PluginFeature {
    fn get_menu(&self) -> Menu {
        match self.menu_state {
            PluginMenuState::PluginSelection => self.get_plugin_selection_menu(),
        }
    }
    
    fn handle_menu_option(&mut self, option_id: Option<&str>, element: Option<&crate::ui::Element>) -> Result<ControllerState> {
        debug!("Plugin feature handle_menu_option called with element: {:?}", element);
        
        // Store the UI element if this is the first call
        if self.ui_element.is_none() && element.is_some() {
            self.ui_element = element.cloned();
        }
        
        // Handle menu closure
        let Some(plugin_uri) = option_id else {
            debug!("Plugin feature: menu closed");
            return Ok(ControllerState::Navigating);
        };
        
        debug!("Plugin feature handling plugin selection: {}", plugin_uri);
        
        // Extract link information from stored element
        let (link_from, link_to) = if let Some(crate::ui::Element::Link(from, to)) = &self.ui_element {
            (from.clone(), to.clone())
        } else {
            return Err(anyhow::anyhow!("Plugin feature requires a link element"));
        };
        
        // Generate a unique block ID based on plugin URI and timestamp
        let block_name = plugin_uri
            .split('/')
            .last()
            .unwrap_or(plugin_uri)
            .replace([':', '.', '#'], "_");
        let block_id = format!("{}_{}", block_name, std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() % 1000000);
        
        debug!("Creating block: {} with plugin: {}", block_id, plugin_uri);
        
        // Create the block in the engine
        self.engine.create_block(plugin_uri, &block_id)?;
        
        let block_path = format!("ingen:/main/{}", block_id);
        
        // Insert node in UI
        self.ui.insert_node(
            block_path.clone(),
            block_id.clone(),
            NodeType::Normal,
            link_from.clone(),
            link_to.clone(),
        )?;
        
        // Find the plugin in the engine's plugin list to get port information
        let plugins = self.engine.list_plugins();
        let plugin = plugins.iter()
            .find(|p| p.id == plugin_uri)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_uri))?;        
        
        // Create connections in the engine (skip "inputs" and "outputs" system nodes)
        if link_from != "inputs" && link_from != "outputs" {
            // Find the first input port of the plugin
            if let Some(input_port) = plugin.ports.iter().find(|p| p.direction == PortDirection::Input) {
                let from_path = link_from.clone();
                let to_path = format!("{}/{}", block_path, input_port.id);
                debug!("Creating engine connection: {} -> {}", from_path, to_path);
                self.engine.connect(&from_path, &to_path)?;
            } else {
                warn!("Plugin {} has no input ports", plugin_uri);
            }
        }
        if link_to != "inputs" && link_to != "outputs" {
            // Find the first output port of the plugin
            if let Some(output_port) = plugin.ports.iter().find(|p| p.direction == PortDirection::Output) {
                let from_path = format!("{}/{}", block_path, output_port.id);
                let to_path = link_to.clone();
                debug!("Creating engine connection: {} -> {}", from_path, to_path);
                self.engine.connect(&from_path, &to_path)?;
            } else {
                warn!("Plugin {} has no output ports", plugin_uri);
            }
        }

        // Disconnect the original connection in the engine (skip system nodes)
        if link_from != "inputs" && link_from != "outputs" && link_to != "inputs" && link_to != "outputs" {
            debug!("Disconnecting original engine connection: {} -> {}", link_from, link_to);
            // Ignore error if connection doesn't exist
            let _ = self.engine.disconnect(&link_from, &link_to);
        }
        
        self.menu_state = PluginMenuState::PluginSelection;
        self.ui_element = None;
        Ok(ControllerState::Navigating)
    }
}

/// Helper to create a new plugin feature
pub fn new_plugin_feature(engine: Arc<Engine>, ui: Arc<UI>) -> PluginFeature {
    PluginFeature::new(engine, ui)
}

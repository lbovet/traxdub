use anyhow::Result;
use log::debug;

use crate::controller::{NavigationLevel, NavigationDirection};

/// UI module - Phase 1: Console-based interface
pub struct UI {}

impl UI {
    /// Create a new UI instance
    pub fn new() -> Self {
        debug!("Initializing UI (console mode)...");
        Self {}
    }

    /// Signal that a plugin instance was created
    pub fn signal_plugin_created(&self, plugin_name: &str, block_id: &str) -> Result<()> {
        println!("[UI] Plugin created: {} ({})", plugin_name, block_id);
        Ok(())
    }

    /// Signal that a connection was made
    pub fn signal_connection_created(&self, source: &str, destination: &str) -> Result<()> {
        println!("[UI] Connection created: {} -> {}", source, destination);
        Ok(())
    }

    /// Signal that a connection was removed
    pub fn signal_connection_removed(&self, source: &str, destination: &str) -> Result<()> {
        println!("[UI] Connection removed: {} -> {}", source, destination);
        Ok(())
    }

    /// Signal a state change
    pub fn signal_state_change(&self, state: &str) -> Result<()> {
        println!("[UI] State changed: {}", state);
        Ok(())
    }

    /// Handle navigation event
    pub fn navigate(&self, level: NavigationLevel, direction: NavigationDirection) -> Result<()> {
        let level_str = match level {
            NavigationLevel::Main => "MAIN",
            NavigationLevel::Secondary => "SECONDARY",
        };
        let direction_str = match direction {
            NavigationDirection::Forward => "FORWARD",
            NavigationDirection::Backward => "BACKWARD",
        };
        println!("[UI] Navigation: {} {}", level_str, direction_str);
        Ok(())
    }

    /// Prompt user to turn the main selection knob
    pub fn prompt_turn_selection_knob(&self) -> Result<()> {
        println!("\n=================================================");
        println!("LEARNING MODE: Base Control Configuration");
        println!("=================================================");
        println!("Please turn the MAIN KNOB");
        println!("(This will be used for navigation and menus)");
        println!("-------------------------------------------------");
        Ok(())
    }

    /// Prompt user to turn the secondary knob
    pub fn prompt_turn_secondary_knob(&self) -> Result<()> {
        println!("\n-------------------------------------------------");
        println!("Great! Now turn the SECONDARY KNOB");
        println!("(This will be used for navigation)");
        println!("-------------------------------------------------");
        Ok(())
    }

    /// Prompt user to press the main selection button
    pub fn prompt_press_selection_button(&self) -> Result<()> {
        println!("\n-------------------------------------------------");
        println!("Perfect! Now press the SELECTION BUTTON");
        println!("(This will be used for selecting items)");
        println!("-------------------------------------------------");
        Ok(())
    }

    /// Prompt user to press the main back button
    pub fn prompt_press_back_button(&self) -> Result<()> {
        println!("\n-------------------------------------------------");
        println!("Excellent! Finally, press the BACK BUTTON");
        println!("(This will be used for going back in menus)");
        println!("-------------------------------------------------");
        Ok(())
    }

    /// Display a general message
    pub fn display_message(&self, message: &str) -> Result<()> {
        println!("[UI] {}", message);
        Ok(())
    }

    /// Display an error message
    pub fn display_error(&self, error: &str) -> Result<()> {
        eprintln!("[UI ERROR] {}", error);
        Ok(())
    }

    /// Display available plugins
    pub fn display_available_plugins(&self, plugins: &[(String, String)]) -> Result<()> {
        println!("\n=================================================");
        println!("Available Plugins:");
        println!("=================================================");
        for (i, (name, uri)) in plugins.iter().enumerate() {
            println!("{}. {} - {}", i + 1, name, uri);
        }
        println!("=================================================\n");
        Ok(())
    }

    /// Display current blocks
    pub fn display_blocks(&self, blocks: &[(String, String)]) -> Result<()> {
        println!("\n=================================================");
        println!("Current Blocks:");
        println!("=================================================");
        if blocks.is_empty() {
            println!("No blocks created yet.");
        } else {
            for (id, plugin_uri) in blocks {
                println!("- {} ({})", id, plugin_uri);
            }
        }
        println!("=================================================\n");
        Ok(())
    }

    /// Display current connections
    pub fn display_connections(&self, connections: &[(String, String)]) -> Result<()> {
        println!("\n=================================================");
        println!("Current Connections:");
        println!("=================================================");
        if connections.is_empty() {
            println!("No connections made yet.");
        } else {
            for (source, dest) in connections {
                println!("- {} -> {}", source, dest);
            }
        }
        println!("=================================================\n");
        Ok(())
    }

    /// Display a menu
    pub fn display_menu(&self, title: &str, items: &[String]) -> Result<()> {
        println!("\n=================================================");
        println!("{}", title);
        println!("=================================================");
        for (i, item) in items.iter().enumerate() {
            println!("{}. {}", i + 1, item);
        }
        println!("=================================================\n");
        Ok(())
    }

    /// Display a parameter change
    pub fn signal_parameter_changed(
        &self,
        block_id: &str,
        parameter: &str,
        value: f32,
    ) -> Result<()> {
        println!(
            "[UI] Parameter changed: {}.{} = {:.3}",
            block_id, parameter, value
        );
        Ok(())
    }
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_creation() {
        let ui = UI::new();
        assert!(ui.signal_state_change("Test").is_ok());
    }

    #[test]
    fn test_plugin_signals() {
        let ui = UI::new();
        assert!(ui.signal_plugin_created("Reverb", "reverb1").is_ok());
        assert!(ui
            .signal_connection_created("source:out", "dest:in")
            .is_ok());
    }
}

use anyhow::Result;
use log::debug;

use crate::controller::{NavigationLevel, KnobDirection};

/// UI module - Phase 1: Console-based interface
pub struct UI {}

impl UI {
    /// Create a new UI instance
    pub fn new() -> Self {
        debug!("Initializing UI (console mode)...");
        Self {}
    }

    /// Signal a state change
    pub fn update_state(&self, state: &str) -> Result<()> {
        println!("[UI] State changed: {}", state);
        Ok(())
    }

    /// Handle navigation event
    pub fn navigate(&self, level: NavigationLevel, direction: KnobDirection) -> Result<()> {
        let level_str = match level {
            NavigationLevel::Main => "MAIN",
            NavigationLevel::Secondary => "SECONDARY",
        };
        let direction_str = match direction {
            KnobDirection::Forward => "FORWARD",
            KnobDirection::Backward => "BACKWARD",
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
        assert!(ui.update_state("Test").is_ok());
    }
}

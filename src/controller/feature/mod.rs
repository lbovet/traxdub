pub mod input;

use anyhow::Result;
use crate::ui::Menu;
use crate::controller::ControllerState;

/// Feature interface for extending controller functionality
pub trait Feature {
    /// Get the menu for this feature
    fn get_menu(&self) -> Menu;
    
    /// Handle menu option selection and return the next controller state
    fn handle_menu_option(&mut self, option_id: &str) -> Result<ControllerState>;
    
    /// Reset the feature to its initial state
    fn reset(&mut self);
}

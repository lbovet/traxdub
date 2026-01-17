pub mod system;

// Re-export input and output features from system module
pub use system::{InputFeature, OutputFeature, new_input_feature, new_output_feature};

use anyhow::Result;
use crate::ui::Menu;
use crate::controller::ControllerState;

/// Feature interface for extending controller functionality
pub trait Feature {
    /// Get the menu for this feature
    fn get_menu(&self) -> Menu;
    
    /// Handle menu option selection and return the next controller state
    /// If option_id is None, the top-most menu was closed and the feature should revert to previous state
    fn handle_menu_option(&mut self, option_id: Option<&str>) -> Result<ControllerState>;
}

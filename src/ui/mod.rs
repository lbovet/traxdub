pub mod window;

use anyhow::Result;
use log::{debug};
use std::sync::Mutex;

use crate::controller::{NavigationLevel, KnobDirection};

/// Menu option
#[derive(Debug, Clone)]
pub struct MenuOption {
    pub id: String,
    pub label: String,
}

/// Menu
#[derive(Debug, Clone)]
pub struct Menu {
    pub id: String,
    pub label: String,
    pub options: Vec<MenuOption>,
}

/// Focused element type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Element {
    Node(String),
    Link(String, String, LinkType), // (from_id, to_id, link_type)
    MenuOption(String, String), // (menu_id, option_id)
}

/// Node type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Normal, 
    PortIn,
    PortOut,   
    Context,
}

/// Link type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkType {
    Normal,
    PortIn,
    PortOut,
    Virtual,
}

/// UI node
#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub node_type: NodeType,
}

/// Link between two nodes
#[derive(Debug, Clone)]
pub struct Link {
    pub from_id: String,
    pub to_id: String,
    pub visited_last: i64,
    pub order: i64,
    pub link_type: LinkType,
}

/// UI module - Phase 1: Console-based interface
pub struct UI {
    session_name: Mutex<Option<String>>, // Current session mnemonic
}

impl UI {
    /// Create a new UI instance
    pub fn new() -> Self {
        debug!("Initializing UI (console mode)...");
        Self {
            session_name: Mutex::new(None),
        }
    }

    /// Create a new node
    pub fn create_node(&self, id: String, name: String, node_type: NodeType) -> Result<()> {
        // TODO: implement
        Ok(())
    }


    /// Create a link between two nodes
    pub fn create_link(&self, from_id: String, to_id: String, link_type: LinkType) -> Result<()> {
        // TODO: implement
        Ok(())
    }

    /// Insert a node between two nodes (connected by a link)
    /// Creates the node and links from link_from to the node and from the node to link_to
    /// Removes the original link, unless it connects "inputs" to "outputs"
    pub fn insert_node(
        &self,
        node_id: String,
        node_name: String,
        node_type: NodeType,
        link_from: String,
        link_to: String,
    ) -> Result<()> {
        // TODO: implement        
        Ok(())
    }

    /// Handle navigation event
    pub fn navigate(&self, level: NavigationLevel, direction: KnobDirection) -> Result<()> {
        // TODO: implement
        Ok(())
    }


    /// Select the currently focused element
    pub fn select(&self) -> Result<Option<Element>> {
        // If a menu is open, return the focused menu option
        // Otherwise, return the focused element        
        let element = Element::MenuOption("dummy_menu".to_string(), "dummy_option".to_string());
        Ok(Some(element))
    }

    /// Display a menu
    fn display_menu(&self, menu: &Menu, focused_index: usize) {
        // TODO: implement
    }

    /// Open a menu and push it onto the menu stack
    pub fn open_menu(&self, menu: Menu) -> Result<()> {
        // TODO: implement
        Ok(())
    }

    /// Close the top-most menu
    pub fn close_menu(&self) -> Result<()> {
        // TODO: implement
        Ok(())
    }

    /// Back: Close the top-most menu and return to previous state
    pub fn back(&self) -> Result<bool> {
        // TODO: implement
        Ok(false) // Return true if we actually went back, false if there was no menu
    }

    /// Close all open menus
    pub fn close_all_menus(&self) -> Result<()> {
        // TODO: implement
        Ok(())
    }

    /// Get the current menu stack size
    pub fn menu_stack_size(&self) -> usize {
        // TODO: implement
        0
    }

    /// Prompt user to turn the main selection knob
    pub fn prompt_turn_selection_knob(&self) -> Result<()> {
        // TODO: implement
        Ok(())
    }

    /// Prompt user to turn the secondary knob
    pub fn prompt_turn_secondary_knob(&self) -> Result<()> {
        // TODO: implement
        Ok(())
    }

    /// Prompt user to press the main selection button
    pub fn prompt_press_selection_button(&self) -> Result<()> {
        // TODO: implement
        Ok(())
    }

    /// Prompt user to press the main back button
    pub fn prompt_press_back_button(&self) -> Result<()> {
        // TODO: implement
        Ok(())
    }
    
    /// Set the current session name (mnemonic)
    pub fn set_session_name(&self, name: String) -> Result<()> {
        debug!("Setting session name: {}", name);
        *self.session_name.lock().unwrap() = Some(name);
        Ok(())
    }
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}

pub mod window;

use anyhow::{Result, Context};
use log::{debug, trace};
use serde_json::json;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

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
    message_queue: Arc<Mutex<VecDeque<String>>>,
    menu_stack_size: Arc<Mutex<usize>>,
}

impl UI {
    /// Create a new UI instance
    pub fn new() -> Self {
        debug!("Initializing UI with IPC message queue...");
        let message_queue = Arc::new(Mutex::new(VecDeque::new()));
        let menu_stack_size = Arc::new(Mutex::new(0));
        
        Self {
            session_name: Mutex::new(None),
            message_queue,
            menu_stack_size,
        }
    }
    
    /// Get the message queue for passing to window::run
    pub fn get_message_queue(&self) -> Arc<Mutex<VecDeque<String>>> {
        Arc::clone(&self.message_queue)
    }
    
    /// Get the menu stack size tracker for passing to window::run
    pub fn get_menu_stack_size(&self) -> Arc<Mutex<usize>> {
        Arc::clone(&self.menu_stack_size)
    }

    /// Send a command to the JavaScript UI
    fn send_command(&self, msg_type: &str, data: serde_json::Value) -> Result<()> {
        let message = json!({
            "type": msg_type,
            "data": data
        });
        let msg_str = serde_json::to_string(&message)
            .context("Failed to serialize UI command")?;
        
        trace!("Queuing UI command: {}", msg_type);
        self.message_queue
            .lock()
            .unwrap()
            .push_back(msg_str);
        Ok(())
    }

    /// Create a new node
    pub fn create_node(&self, id: String, name: String, node_type: NodeType) -> Result<()> {
        anyhow::ensure!(!id.is_empty(), "Node ID cannot be empty");
        trace!("Creating node: {} ({})", name, id);
        
        let node_type_str = match node_type {
            NodeType::Normal => "normal",
            NodeType::PortIn => "portIn",
            NodeType::PortOut => "portOut",
            NodeType::Context => "context",
        };
        
        self.send_command("create_node", json!({
            "id": id,
            "label": name,
            "nodeType": node_type_str
        }))
    }


    /// Create a link between two nodes
    pub fn create_link(&self, from_id: String, to_id: String, link_type: LinkType) -> Result<()> {
        anyhow::ensure!(!from_id.is_empty(), "From ID cannot be empty");
        anyhow::ensure!(!to_id.is_empty(), "To ID cannot be empty");
        trace!("Creating link: {} -> {}", from_id, to_id);
        
        let link_type_str = match link_type {
            LinkType::Normal => "normal",
            LinkType::PortIn => "portIn",
            LinkType::PortOut => "portOut",
            LinkType::Virtual => "virtual",
        };
        
        self.send_command("create_link", json!({
            "fromId": from_id,
            "toId": to_id,
            "linkType": link_type_str
        }))
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
        anyhow::ensure!(!node_id.is_empty(), "Node ID cannot be empty");
        anyhow::ensure!(!link_from.is_empty(), "Link from ID cannot be empty");
        anyhow::ensure!(!link_to.is_empty(), "Link to ID cannot be empty");
        trace!("Inserting node {} between {} and {}", node_name, link_from, link_to);
        
        let node_type_str = match node_type {
            NodeType::Normal => "normal",
            NodeType::PortIn => "portIn",
            NodeType::PortOut => "portOut",
            NodeType::Context => "context",
        };
        
        self.send_command("insert_node", json!({
            "id": node_id,
            "label": node_name,
            "nodeType": node_type_str,
            "linkFrom": link_from,
            "linkTo": link_to
        }))
    }

    /// Handle navigation event
    pub fn navigate(&self, level: NavigationLevel, direction: KnobDirection) -> Result<()> {
        trace!("Navigate: {:?} {:?}", level, direction);
        
        let level_str = match level {
            NavigationLevel::Main => "main",
            NavigationLevel::Secondary => "secondary",
        };
        
        let direction_str = match direction {
            KnobDirection::Forward => "forward",
            KnobDirection::Backward => "backward",
        };
        
        self.send_command("navigate", json!({
            "level": level_str,
            "direction": direction_str
        }))
    }


    /// Select the currently focused element
    pub fn select(&self) -> Result<Option<Element>> {
        // If a menu is open, return the focused menu option
        // Otherwise, return the focused element        
        let element = Element::MenuOption("dummy_menu".to_string(), "dummy_option".to_string());
        Ok(Some(element))
    }

    /// Open a menu and push it onto the menu stack
    pub fn open_menu(&self, menu: Menu) -> Result<()> {
        trace!("Opening menu: {}", menu.label);
        
        let options: Vec<_> = menu.options.iter()
            .map(|opt| json!({
                "id": opt.id,
                "label": opt.label
            }))
            .collect();
        
        self.send_command("open_menu", json!({
            "id": menu.id,
            "label": menu.label,
            "options": options
        }))?;
        
        // Increment menu stack size
        *self.menu_stack_size.lock().unwrap() += 1;
        Ok(())
    }

    /// Close the top-most menu
    pub fn close_menu(&self) -> Result<()> {
        trace!("Closing menu");
        
        let mut size = self.menu_stack_size.lock().unwrap();
        if *size > 0 {
            *size -= 1;
            self.send_command("close_menu", json!({}))
        } else {
            Ok(())
        }
    }

    /// Back: Close the top-most menu and return to previous state
    pub fn back(&self) -> Result<bool> {
        let size = *self.menu_stack_size.lock().unwrap();
        
        if size > 0 {
            trace!("Going back (menu stack size: {})", size);
            self.close_menu()?;
            Ok(true)
        } else {
            trace!("Back requested but no menu open");
            Ok(false)
        }
    }

    /// Close all open menus
    pub fn close_all_menus(&self) -> Result<()> {
        trace!("Closing all menus");
        
        *self.menu_stack_size.lock().unwrap() = 0;
        self.send_command("close_all_menus", json!({}))
    }

    /// Get the current menu stack size
    pub fn menu_stack_size(&self) -> usize {
        *self.menu_stack_size.lock().unwrap()
    }

    /// Prompt user to turn the main selection knob
    pub fn prompt_turn_selection_knob(&self) -> Result<()> {
        trace!("Prompt: turn selection knob");
        self.send_command("prompt", json!({
            "message": "Turn the main selection knob"
        }))
    }

    /// Prompt user to turn the secondary knob
    pub fn prompt_turn_secondary_knob(&self) -> Result<()> {
        trace!("Prompt: turn secondary knob");
        self.send_command("prompt", json!({
            "message": "Turn the secondary knob"
        }))
    }

    /// Prompt user to press the main selection button
    pub fn prompt_press_selection_button(&self) -> Result<()> {
        trace!("Prompt: press selection button");
        self.send_command("prompt", json!({
            "message": "Press the main selection button"
        }))
    }

    /// Prompt user to press the main back button
    pub fn prompt_press_back_button(&self) -> Result<()> {
        trace!("Prompt: press back button");
        self.send_command("prompt", json!({
            "message": "Press the main back button"
        }))
    }
    
    /// Commit pending visual changes
    pub fn commit(&self) -> Result<()> {
        trace!("Committing visual changes");
        self.send_command("commit", json!({}))
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

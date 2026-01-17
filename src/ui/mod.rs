use anyhow::Result;
use log::{debug, trace};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::controller::{NavigationLevel, KnobDirection};

/// Focused element type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Element {
    Node(String),
    Link(String, String),
}

/// Node type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Normal,
    System,
}

/// UI node
#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
}

/// Link between two nodes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Link {
    pub from_id: String,
    pub to_id: String,
}

/// UI module - Phase 1: Console-based interface
pub struct UI {
    nodes: Mutex<HashMap<String, Node>>,
    links: Mutex<HashSet<Link>>,
    focused_element: Mutex<Option<Element>>,
}

impl UI {
    /// Create a new UI instance
    pub fn new() -> Self {
        debug!("Initializing UI (console mode)...");
        Self {
            nodes: Mutex::new(HashMap::new()),
            links: Mutex::new(HashSet::new()),
            focused_element: Mutex::new(None),
        }
    }

    /// Create a new node
    pub fn create_node(&self, id: String, name: String, node_type: NodeType) -> Result<()> {
        debug!("Creating node: id={}, name={}, type={:?}", id, name, node_type);
        
        let node = Node {
            id: id.clone(),
            name,
            node_type,
        };
        
        self.nodes.lock().unwrap().insert(id.clone(), node);
        
        // Set focus to first node if nothing is focused yet
        let mut focused = self.focused_element.lock().unwrap();
        if focused.is_none() {
            *focused = Some(Element::Node(id.clone()));
            debug!("Setting initial focus to node: {}", id);
        }
        drop(focused);
        
        self.display_graph();
        Ok(())
    }

    /// Remove an node by id
    pub fn remove_node(&self, id: &str) -> Result<()> {
        debug!("Removing node: id={}", id);
        
        if self.nodes.lock().unwrap().remove(id).is_some() {
            self.display_graph();
            Ok(())
        } else {
            Err(anyhow::anyhow!("node not found: {}", id))
        }
    }

    /// Get an node by id
    pub fn get_node(&self, id: &str) -> Option<Node> {
        self.nodes.lock().unwrap().get(id).cloned()
    }

    /// Create a link between two nodes
    pub fn create_link(&self, from_id: String, to_id: String) -> Result<()> {
        debug!("Creating link: {} -> {}", from_id, to_id);
        
        // Verify both nodes exist
        let nodes = self.nodes.lock().unwrap();
        if !nodes.contains_key(&from_id) {
            return Err(anyhow::anyhow!("Source node not found: {}", from_id));
        }
        if !nodes.contains_key(&to_id) {
            return Err(anyhow::anyhow!("Destination node not found: {}", to_id));
        }
        drop(nodes);
        
        let link = Link { from_id, to_id };
        self.links.lock().unwrap().insert(link);
        self.display_graph();
        Ok(())
    }

    /// Remove a link between two nodes
    pub fn remove_link(&self, from_id: &str, to_id: &str) -> Result<()> {
        debug!("Removing link: {} -> {}", from_id, to_id);
        
        let link = Link {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
        };
        
        if self.links.lock().unwrap().remove(&link) {
            self.display_graph();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Link not found: {} -> {}", from_id, to_id))
        }
    }

    /// Get all links
    pub fn get_all_links(&self) -> Vec<Link> {
        self.links.lock().unwrap().iter().cloned().collect()
    }

    /// Display the current graph of nodes and links
    fn display_graph(&self) {
        println!("\n=== Graph State ===");
        
        let nodes = self.nodes.lock().unwrap();
        let links = self.links.lock().unwrap();
        let focused = self.focused_element.lock().unwrap();
        
        // Get sorted list of node IDs
        let mut node_ids: Vec<&String> = nodes.keys().collect();
        node_ids.sort();
        
        for node_id in node_ids {
            // Indicate if this is the focused node
            let focus_marker = if let Some(Element::Node(ref focused_id)) = *focused {
                if focused_id == node_id { " [*]" } else { "" }
            } else {
                ""
            };
            
            println!("{}{}", node_id, focus_marker);
            
            // Find all links FROM this node
            let from_links: Vec<&Link> = links
                .iter()
                .filter(|link| &link.from_id == node_id)
                .collect();
            
            if !from_links.is_empty() {
                for link in from_links {
                    // Check if this link is focused
                    let link_focus_marker = if let Some(Element::Link(ref from, ref to)) = *focused {
                        if from == &link.from_id && to == &link.to_id { " [*]" } else { "" }
                    } else {
                        ""
                    };
                    println!("   →{} {}", link_focus_marker, link.to_id);
                }
            }
        }
        
        println!("==================\n");
    }

    /// Signal a state change
    pub fn update_state(&self, state: &str) -> Result<()> {
        println!("[UI] State changed: {}", state);
        Ok(())
    }

    /// Handle navigation event
    pub fn navigate(&self, level: NavigationLevel, direction: KnobDirection) -> Result<()> {
        match level {
            NavigationLevel::Main => {
                let mut focused = self.focused_element.lock().unwrap();
                let links = self.links.lock().unwrap();
                
                // Clone the current focus to avoid borrow checker issues
                let current_focus = focused.clone();
                
                match current_focus {
                    Some(Element::Node(current_node_id)) => {
                        // Navigate from a node to a link
                        match direction {
                            KnobDirection::Forward => {
                                // Find first link FROM current node
                                if let Some(link) = links.iter().find(|link| &link.from_id == &current_node_id) {
                                    trace!("Navigating from node {} to link {} -> {}", current_node_id, link.from_id, link.to_id);
                                    *focused = Some(Element::Link(link.from_id.clone(), link.to_id.clone()));
                                    println!("[UI] Focus moved to link: {} → {}", link.from_id, link.to_id);
                                }
                            }
                            KnobDirection::Backward => {
                                // Find first link TO current node
                                if let Some(link) = links.iter().find(|link| &link.to_id == &current_node_id) {
                                    trace!("Navigating from node {} to link {} -> {}", current_node_id, link.from_id, link.to_id);
                                    *focused = Some(Element::Link(link.from_id.clone(), link.to_id.clone()));
                                    println!("[UI] Focus moved to link: {} → {}", link.from_id, link.to_id);
                                }
                            }
                        }
                    }
                    Some(Element::Link(from_id, to_id)) => {
                        // Navigate from a link to a node
                        match direction {
                            KnobDirection::Forward => {
                                // Move to destination node
                                trace!("Navigating from link {} -> {} to node {}", from_id, to_id, to_id);
                                *focused = Some(Element::Node(to_id.clone()));
                                println!("[UI] Focus moved to node: {}", to_id);
                            }
                            KnobDirection::Backward => {
                                // Move to source node
                                trace!("Navigating from link {} -> {} to node {}", from_id, to_id, from_id);
                                *focused = Some(Element::Node(from_id.clone()));
                                println!("[UI] Focus moved to node: {}", from_id);
                            }
                        }
                    }
                    None => {
                        println!("[UI] No element currently focused");
                    }
                }
            }
            NavigationLevel::Secondary => {
                let direction_str = match direction {
                    KnobDirection::Forward => "FORWARD",
                    KnobDirection::Backward => "BACKWARD",
                };
                println!("[UI] Navigation: SECONDARY {}", direction_str);
            }
        }
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

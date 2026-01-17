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
#[derive(Debug, Clone)]
pub struct Link {
    pub from_id: String,
    pub to_id: String,
    pub visited_last: i64,
    pub order: i64,
}

// Implement PartialEq to only compare from_id and to_id (not visited_last or order)
impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        self.from_id == other.from_id && self.to_id == other.to_id
    }
}

impl Eq for Link {}

// Implement Hash to only hash from_id and to_id (not visited_last or order)
impl std::hash::Hash for Link {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.from_id.hash(state);
        self.to_id.hash(state);
    }
}

/// UI module - Phase 1: Console-based interface
pub struct UI {
    nodes: Mutex<HashMap<String, Node>>,
    links: Mutex<HashSet<Link>>,
    focused_element: Mutex<Option<Element>>,
    visit_counter: Mutex<i64>,
    order_counter: Mutex<i64>,
}

impl UI {
    /// Create a new UI instance
    pub fn new() -> Self {
        debug!("Initializing UI (console mode)...");
        Self {
            nodes: Mutex::new(HashMap::new()),
            links: Mutex::new(HashSet::new()),
            focused_element: Mutex::new(None),
            visit_counter: Mutex::new(0),
            order_counter: Mutex::new(0),
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
        
        // Get and increment order counter
        let mut order_counter = self.order_counter.lock().unwrap();
        *order_counter += 1;
        let link_order = *order_counter;
        drop(order_counter);
        
        let link = Link { 
            from_id, 
            to_id,
            visited_last: -link_order,
            order: link_order,
        };
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
            visited_last: 0, // Value doesn't matter for lookup since Hash/Eq ignore it
            order: 0, // Value doesn't matter for lookup since Hash/Eq ignore it
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
            
            // Find all links FROM this node and sort by order
            let mut from_links: Vec<&Link> = links
                .iter()
                .filter(|link| &link.from_id == node_id)
                .collect();
            from_links.sort_by_key(|link| link.order);
            
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
                        // Drop the immutable borrow before getting mutable borrow
                        drop(links);
                        let mut links_mut = self.links.lock().unwrap();
                        
                        match direction {
                            KnobDirection::Forward => {
                                // Find link FROM current node with highest visited_last
                                if let Some(link) = links_mut.iter()
                                    .filter(|link| &link.from_id == &current_node_id)
                                    .max_by_key(|link| link.visited_last) {
                                    
                                    // Update visited_last: remove, update, re-insert
                                    let mut updated_link = link.clone();
                                    links_mut.remove(&updated_link);
                                    
                                    let mut counter = self.visit_counter.lock().unwrap();
                                    *counter += 1;
                                    updated_link.visited_last = *counter;
                                    drop(counter);
                                    
                                    trace!("Navigating from node {} to link {} -> {} (visit={})", 
                                           current_node_id, updated_link.from_id, updated_link.to_id, updated_link.visited_last);
                                    
                                    *focused = Some(Element::Link(updated_link.from_id.clone(), updated_link.to_id.clone()));
                                    println!("[UI] Focus moved to link: {} → {}", updated_link.from_id, updated_link.to_id);
                                    
                                    links_mut.insert(updated_link);
                                }
                            }
                            KnobDirection::Backward => {
                                // Find link TO current node with highest visited_last
                                if let Some(link) = links_mut.iter()
                                    .filter(|link| &link.to_id == &current_node_id)
                                    .max_by_key(|link| link.visited_last) {
                                    
                                    // Update visited_last: remove, update, re-insert
                                    let mut updated_link = link.clone();
                                    links_mut.remove(&updated_link);
                                    
                                    let mut counter = self.visit_counter.lock().unwrap();
                                    *counter += 1;
                                    updated_link.visited_last = *counter;
                                    drop(counter);
                                    
                                    trace!("Navigating from node {} to link {} -> {} (visit={})", 
                                           current_node_id, updated_link.from_id, updated_link.to_id, updated_link.visited_last);
                                    
                                    *focused = Some(Element::Link(updated_link.from_id.clone(), updated_link.to_id.clone()));
                                    println!("[UI] Focus moved to link: {} → {}", updated_link.from_id, updated_link.to_id);
                                    
                                    links_mut.insert(updated_link);
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
                let mut focused = self.focused_element.lock().unwrap();
                let current_focus = focused.clone();
                
                match current_focus {
                    Some(Element::Link(from_id, to_id)) => {
                        // Navigate between links with the same from_id
                        let links = self.links.lock().unwrap();
                        
                        // Find current link to get its order
                        if let Some(current_link) = links.iter().find(|l| l.from_id == from_id && l.to_id == to_id) {
                            let current_order = current_link.order;
                            
                            // Get all links with same from_id, excluding current link
                            let sibling_links: Vec<&Link> = links.iter()
                                .filter(|l| l.from_id == from_id && !(l.from_id == from_id && l.to_id == to_id))
                                .collect();
                            
                            if let Some(link) = Self::find_sibling_link(&sibling_links, current_order, &direction) {
                                trace!("Navigating from link {}→{} (order={}) to link {}→{} (order={})",
                                       from_id, to_id, current_order, link.from_id, link.to_id, link.order);
                                *focused = Some(Element::Link(link.from_id.clone(), link.to_id.clone()));
                                println!("[UI] Focus moved to link: {} → {}", link.from_id, link.to_id);
                            } 
                        }
                    }
                    Some(Element::Node(node_id)) => {
                        // Navigate from a node: find most recently visited incoming link,
                        // then find its sibling and focus the node it targets
                        let links = self.links.lock().unwrap();
                        
                        // Find the most recently visited link targeting this node
                        if let Some(most_recent_link) = links.iter()
                            .filter(|l| l.to_id == node_id)
                            .max_by_key(|l| l.visited_last) {
                            
                            let current_order = most_recent_link.order;
                            let from_id = &most_recent_link.from_id;
                            
                            // Get all links with same from_id, excluding the most recent one
                            let sibling_links: Vec<&Link> = links.iter()
                                .filter(|l| &l.from_id == from_id && !(l.from_id == most_recent_link.from_id && l.to_id == most_recent_link.to_id))
                                .collect();
                            
                            if let Some(sibling_link) = Self::find_sibling_link(&sibling_links, current_order, &direction) {
                                let target_node = &sibling_link.to_id;
                                trace!("Navigating from node {} via sibling link to node {}", node_id, target_node);
                                *focused = Some(Element::Node(target_node.clone()));
                                println!("[UI] Focus moved to node: {}", target_node);
                            }
                        }
                    }
                    None => {
                        println!("[UI] No element currently focused");
                    }
                }
            }
        }
        Ok(())
    }

    /// Find a sibling link based on order and navigation direction
    /// Returns the link with the closest order in the specified direction
    fn find_sibling_link<'a>(
        sibling_links: &[&'a Link],
        current_order: i64,
        direction: &KnobDirection,
    ) -> Option<&'a Link> {
        match direction {
            KnobDirection::Forward => {
                // Find link with smallest order greater than current
                sibling_links.iter()
                    .filter(|l| l.order > current_order)
                    .min_by_key(|l| l.order)
                    .copied()
            }
            KnobDirection::Backward => {
                // Find link with largest order less than current
                sibling_links.iter()
                    .filter(|l| l.order < current_order)
                    .max_by_key(|l| l.order)
                    .copied()
            }
        }
    }

    /// Select the currently focused element
    pub fn select(&self) -> Result<Option<Element>> {
        let focused = self.focused_element.lock().unwrap();
        let element = focused.clone();
        
        if let Some(ref elem) = element {
            match elem {
                Element::Node(id) => {
                    println!("[UI] Selected node: {}", id);
                }
                Element::Link(from_id, to_id) => {
                    println!("[UI] Selected link: {} → {}", from_id, to_id);
                }
            }
        } else {
            println!("[UI] No element focused to select");
        }
        
        drop(focused);
        self.display_graph();
        
        Ok(element)
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

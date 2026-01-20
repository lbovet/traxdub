use anyhow::Result;
use log::{debug, trace};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::controller::{NavigationLevel, KnobDirection};

/// Menu option
#[derive(Debug, Clone)]
pub struct MenuOption {
    pub id: String,
    pub name: String,
}

/// Menu
#[derive(Debug, Clone)]
pub struct Menu {
    pub id: String,
    pub name: String,
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
    pub link_type: LinkType,
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
    menu_stack: Mutex<Vec<Menu>>,
    focused_menu_option: Mutex<Option<usize>>, // Index into current menu's options
    menu_focus_memory: Mutex<HashMap<String, usize>>, // Remember last focused option for each menu ID
    session_name: Mutex<Option<String>>, // Current session mnemonic
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
            menu_stack: Mutex::new(Vec::new()),
            focused_menu_option: Mutex::new(None),
            menu_focus_memory: Mutex::new(HashMap::new()),
            session_name: Mutex::new(None),
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
        
        // Set focus to the new node if navigable (check before moving)
        let should_focus = node.node_type != NodeType::Context;
        
        self.nodes.lock().unwrap().insert(id.clone(), node);
        
        if should_focus {
            let mut focused = self.focused_element.lock().unwrap();
            *focused = Some(Element::Node(id.clone()));
            drop(focused);
        }
        
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
    pub fn create_link(&self, from_id: String, to_id: String, link_type: LinkType) -> Result<()> {
        debug!("Creating link: {} -> {} (type: {:?})", from_id, to_id, link_type);
        
        // Verify both nodes exist and check if they are context nodes
        let nodes = self.nodes.lock().unwrap();
        if !nodes.contains_key(&from_id) {
            return Err(anyhow::anyhow!("Source node not found: {}", from_id));
        }
        if !nodes.contains_key(&to_id) {
            return Err(anyhow::anyhow!("Destination node not found: {}", to_id));
        }
        
        let from_is_context = nodes.get(&from_id).map(|n| n.node_type == NodeType::Context).unwrap_or(false);
        let to_is_context = nodes.get(&to_id).map(|n| n.node_type == NodeType::Context).unwrap_or(false);
        drop(nodes);
        
        // Get and increment order counter
        let mut order_counter = self.order_counter.lock().unwrap();
        *order_counter += 1;
        let link_order = if from_is_context && to_is_context {
            // Links between context nodes get maximum order value
            i64::MAX
        } else {
            *order_counter
        };
        drop(order_counter);
        
        let link = Link { 
            from_id: from_id.clone(), 
            to_id: to_id.clone(),
            visited_last: -link_order,
            order: link_order,
            link_type: link_type.clone(),
        };
        self.links.lock().unwrap().insert(link);
        
        // Focus the new link if nothing is focused yet
        let mut focused = self.focused_element.lock().unwrap();
        if focused.is_none() {
            *focused = Some(Element::Link(from_id, to_id, link_type));
        }
        drop(focused);
        
        self.display_graph();
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
        debug!("Inserting node {} between {} and {}", node_id, link_from, link_to);
        
        // Create the new node
        self.create_node(node_id.clone(), node_name, node_type.clone())?;
        
        // Calculate link types based on node types
        let nodes = self.nodes.lock().unwrap();
        let from_node_type = nodes.get(&link_from).map(|n| n.node_type.clone());
        let to_node_type = nodes.get(&link_to).map(|n| n.node_type.clone());
        drop(nodes);
        
        let link1_type = Self::calculate_link_type(from_node_type.as_ref(), Some(&node_type));
        let link2_type = Self::calculate_link_type(Some(&node_type), to_node_type.as_ref());
        
        // Create links
        self.create_link(link_from.clone(), node_id.clone(), link1_type)?;
        self.create_link(node_id, link_to.clone(), link2_type)?;
        
        // Remove the original link, unless it connects inputs to outputs
        if !(link_from == "inputs" && link_to == "outputs") {
            self.remove_link(&link_from, &link_to)?;
        }
        
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
            link_type: LinkType::Normal, // Value doesn't matter for lookup since Hash/Eq ignore it
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
        let session_name = self.session_name.lock().unwrap();
        if let Some(ref name) = *session_name {
            println!("\n=== Graph State - {} ===", name);
        } else {
            println!("\n=== Graph State ===");
        }
        
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
                    let link_focus_marker = if let Some(Element::Link(ref from, ref to, _)) = *focused {
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

    /// Handle navigation event
    pub fn navigate(&self, level: NavigationLevel, direction: KnobDirection) -> Result<()> {
        // Check if we have a menu open - if so, handle menu navigation
        let menu_stack = self.menu_stack.lock().unwrap();
        if !menu_stack.is_empty() && level == NavigationLevel::Main {
            let current_menu = menu_stack.last().unwrap();
            let option_count = current_menu.options.len();
            drop(menu_stack);
            
            let mut focused_option = self.focused_menu_option.lock().unwrap();
            let current_idx = focused_option.unwrap_or(0);
            
            let new_idx = match direction {
                KnobDirection::Forward => {
                    if current_idx + 1 < option_count {
                        current_idx + 1
                    } else {
                        0 // Wrap to first option
                    }
                }
                KnobDirection::Backward => {
                    if current_idx > 0 {
                        current_idx - 1
                    } else {
                        option_count - 1 // Wrap to last option
                    }
                }
            };
            
            if new_idx != current_idx {
                *focused_option = Some(new_idx);
                drop(focused_option);
                
                // Redisplay menu with new focus
                let menu_stack = self.menu_stack.lock().unwrap();
                if let Some(current_menu) = menu_stack.last() {
                    // Save the focused index for this menu
                    self.menu_focus_memory.lock().unwrap().insert(current_menu.id.clone(), new_idx);
                    self.display_menu(current_menu, new_idx);
                }
                drop(menu_stack);                        
            }
            
            return Ok(());
        }
        drop(menu_stack);
        
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
                                    
                                    *focused = Some(Element::Link(updated_link.from_id.clone(), updated_link.to_id.clone(), updated_link.link_type.clone()));

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
                                    
                                    *focused = Some(Element::Link(updated_link.from_id.clone(), updated_link.to_id.clone(), updated_link.link_type.clone()));
                                    
                                    links_mut.insert(updated_link);
                                }
                            }
                        }
                    }
                    Some(Element::Link(from_id, to_id, _)) => {
                        // Navigate from a link to a node
                        let nodes = self.nodes.lock().unwrap();
                        
                        match direction {
                            KnobDirection::Forward => {
                                // Move to destination node, but skip context nodes
                                if let Some(node) = nodes.get(&to_id) {
                                    if node.node_type != NodeType::Context {
                                        trace!("Navigating from link {} -> {} to node {}", from_id, to_id, to_id);
                                        *focused = Some(Element::Node(to_id.clone()));
                                    } else {
                                        trace!("Skipping context node {} during navigation", to_id);
                                    }
                                }
                            }
                            KnobDirection::Backward => {
                                // Move to source node, but skip context nodes
                                if let Some(node) = nodes.get(&from_id) {
                                    if node.node_type != NodeType::Context {
                                        trace!("Navigating from link {} -> {} to node {}", from_id, to_id, from_id);
                                        *focused = Some(Element::Node(from_id.clone()));
                                    } else {
                                        trace!("Skipping context node {} during navigation", from_id);
                                    }
                                }
                            }
                        }
                    }
                    Some(Element::MenuOption(_, _)) => {
                        // Menu navigation is handled at the top of the function
                        // This case should not be reached
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
                    Some(Element::Link(from_id, to_id, _)) => {
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
                                *focused = Some(Element::Link(link.from_id.clone(), link.to_id.clone(), link.link_type.clone()));
                            } 
                        }
                    }
                    Some(Element::Node(node_id)) => {
                        // Navigate from a node: find most recently visited incoming link,
                        // then find its sibling and focus the node it targets
                        let links = self.links.lock().unwrap();
                        let nodes = self.nodes.lock().unwrap();
                        
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
                                
                                // Check if target node is a context node - skip if it is
                                if let Some(node) = nodes.get(target_node) {
                                    if node.node_type != NodeType::Context {
                                        trace!("Navigating from node {} via sibling link to node {}", node_id, target_node);
                                        *focused = Some(Element::Node(target_node.clone()));
                                    } else {
                                        trace!("Skipping context node {} during secondary navigation", target_node);
                                    }
                                }
                            }
                        }
                    }
                    Some(Element::MenuOption(_, _)) => {
                        // Menu navigation is handled at main level
                        // Secondary navigation is not used for menus
                    }
                    None => {
                        println!("[UI] No element currently focused");
                    }
                }
            }
        }
        self.display_graph();
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
        // If a menu is open, return the focused menu option
        let menu_stack = self.menu_stack.lock().unwrap();
        if let Some(current_menu) = menu_stack.last() {
            let focused_option = self.focused_menu_option.lock().unwrap();
            if let Some(idx) = *focused_option {
                if idx < current_menu.options.len() {
                    let option = &current_menu.options[idx];
                    println!("[UI] Selected menu option: {}", option.name);
                    let element = Element::MenuOption(current_menu.id.clone(), option.id.clone());
                    drop(focused_option);
                    drop(menu_stack);
                    self.display_graph();
                    return Ok(Some(element));
                }
            }
        }
        drop(menu_stack);
        
        // Otherwise, return the focused element
        let focused = self.focused_element.lock().unwrap();
        let element = focused.clone();
        
        if let Some(ref elem) = element {
            match elem {
                Element::Node(id) => {
                    println!("[UI] Selected node: {}", id);
                }
                Element::Link(from_id, to_id, link_type) => {
                    println!("[UI] Selected link: {} → {} (type: {:?})", from_id, to_id, link_type);
                }
                Element::MenuOption(menu_id, option_id) => {
                    println!("[UI] Selected menu option: {} from menu {}", option_id, menu_id);
                }
            }
        } else {
            println!("[UI] No element focused to select");
        }
        
        drop(focused);
        self.display_graph();
        
        Ok(element)
    }

    /// Display a menu to the console
    fn display_menu(&self, menu: &Menu, focused_index: usize) {
        println!("\n=== Menu: {} ===", menu.name);
        for (i, option) in menu.options.iter().enumerate() {
            let marker = if i == focused_index { " [*]" } else { "" };
            println!("{}.{} {}", i + 1, marker, option.name);
        }
        println!("=========================\n");
    }

    /// Open a menu and push it onto the menu stack
    pub fn open_menu(&self, menu: Menu) -> Result<()> {
        debug!("Opening menu: id={}, name={}, options={}", menu.id, menu.name, menu.options.len());
        
        // Check if we have a remembered focus position for this menu
        let memory = self.menu_focus_memory.lock().unwrap();
        let focused_idx = memory.get(&menu.id).copied().unwrap_or(0);
        drop(memory);
        
        // Ensure the focused index is valid for this menu
        let focused_idx = if focused_idx < menu.options.len() { focused_idx } else { 0 };
        
        // Display the menu with remembered focus
        self.display_menu(&menu, focused_idx);
        
        // Set the focused option
        *self.focused_menu_option.lock().unwrap() = Some(focused_idx);
        
        self.menu_stack.lock().unwrap().push(menu);
        Ok(())
    }

    /// Close the top-most menu
    pub fn close_menu(&self) -> Result<()> {
        let mut stack = self.menu_stack.lock().unwrap();
        if let Some(menu) = stack.pop() {
            debug!("Closed menu: id={}, name={}", menu.id, menu.name);
            
            // If there's still a menu in the stack, display it
            if let Some(current_menu) = stack.last() {
                let menu_id = current_menu.id.clone();
                drop(stack);
                
                // Restore previously focused option for this menu
                let memory = self.menu_focus_memory.lock().unwrap();
                let focused_idx = memory.get(&menu_id).copied().unwrap_or(0);
                drop(memory);
                
                *self.focused_menu_option.lock().unwrap() = Some(focused_idx);
                
                // Display the current menu with restored focus
                let stack = self.menu_stack.lock().unwrap();
                if let Some(menu) = stack.last() {
                    // Ensure the focused index is valid for this menu
                    let focused_idx = if focused_idx < menu.options.len() { focused_idx } else { 0 };
                    self.display_menu(menu, focused_idx);
                }
            } else {
                drop(stack);
                
                self.display_graph();

                // Clear focused menu option when no more menus
                *self.focused_menu_option.lock().unwrap() = None;
            }
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("No menu to close"))
        }
    }

    /// Back: Close the top-most menu and return to previous state
    pub fn back(&self) -> Result<bool> {
        let _ = self.close_menu();
        return Ok(self.menu_stack_size() == 0);
    }

    /// Close all open menus
    pub fn close_all_menus(&self) -> Result<()> {
        let mut stack = self.menu_stack.lock().unwrap();
        let count = stack.len();
        stack.clear();
        debug!("Closed all {} menu(s)", count);
        Ok(())
    }

    /// Get the current menu stack size
    pub fn menu_stack_size(&self) -> usize {
        self.menu_stack.lock().unwrap().len()
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

    /// Calculate link type based on source and destination node types
    fn calculate_link_type(from_type: Option<&NodeType>, to_type: Option<&NodeType>) -> LinkType {
        match (from_type, to_type) {
            // Context -> Context => Virtual
            (Some(NodeType::Context), Some(NodeType::Context)) => LinkType::Virtual,
            // Context -> PortIn Node => PortIn
            (Some(NodeType::Context), Some(NodeType::PortIn)) => LinkType::PortIn,
            // PortOut Node -> Context => PortOut
            (Some(NodeType::PortOut), Some(NodeType::Context)) => LinkType::PortOut,
            // Non-PortOut Node -> Context => Virtual
            (Some(NodeType::Normal), Some(NodeType::Context)) |
            (Some(NodeType::PortIn), Some(NodeType::Context)) => LinkType::Virtual,
            // Context -> Non-PortIn Node => Virtual
            (Some(NodeType::Context), Some(NodeType::Normal)) |
            (Some(NodeType::Context), Some(NodeType::PortOut)) => LinkType::Virtual,
            // Other cases: Normal
            _ => LinkType::Normal,
        }
    }

    /// Set the current session name (mnemonic)
    pub fn set_session_name(&self, name: String) -> Result<()> {
        debug!("Setting session name: {}", name);
        *self.session_name.lock().unwrap() = Some(name);
        self.display_graph();
        Ok(())
    }
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}

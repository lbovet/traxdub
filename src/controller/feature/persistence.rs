use anyhow::Result;
use log::{debug, info, warn};
use std::sync::Arc;
use std::fs;
use std::path::PathBuf;
use chrono::{Local, TimeZone};

use crate::controller::{ControllerState, feature::Feature};
use crate::controller::driver::{Driver, PortType};
use crate::engine::Engine;
use crate::ui::{Menu, MenuOption, UI};

/// Menu state for the persistence feature
#[derive(Debug, Clone, PartialEq)]
enum PersistenceMenuState {
    FileMenu,
    LoadSelection,
    TimestampSelection(String), // mnemonic
}

/// Persistence feature for saving and loading engine state
pub struct PersistenceFeature {
    driver: Arc<Driver>,
    engine: Arc<Engine>,
    ui: Arc<UI>,
    menu_state: PersistenceMenuState,
    current_mnemonic: Option<String>,
}

impl PersistenceFeature {
    /// Create a new persistence feature
    pub fn new(driver: Arc<Driver>, engine: Arc<Engine>, ui: Arc<UI>, auto_load: bool) -> Self {
        let mut feature = Self {
            driver,
            engine,
            ui,
            menu_state: PersistenceMenuState::FileMenu,
            current_mnemonic: None,
        };
        
        // Auto-load most recent save if requested
        if auto_load {
            if let Err(e) = feature.load_most_recent() {
                info!("Could not auto-load most recent save: {}", e);
            }
        }
        
        feature
    }
    
    /// Load the most recent saved state
    fn load_most_recent(&mut self) -> Result<()> {
        // Get all saved files
        let store_dir = Self::get_store_dir()?;
        
        if !store_dir.exists() {
            return Err(anyhow::anyhow!("No saved sessions found"));
        }
        
        let mut files: Vec<(String, String)> = Vec::new();
        
        for entry in fs::read_dir(store_dir)? {
            let entry = entry?;
            let filename = entry.file_name().to_string_lossy().to_string();
            
            if let Some((timestamp, mnemonic)) = Self::parse_filename(&filename) {
                files.push((timestamp, mnemonic));
            }
        }
        
        if files.is_empty() {
            return Err(anyhow::anyhow!("No saved sessions found"));
        }
        
        // Sort by timestamp descending to get most recent
        files.sort_by(|a, b| b.0.cmp(&a.0));
        
        let (timestamp, mnemonic) = &files[0];
        
        info!("Auto-loading most recent session: {} ({})", 
              Self::format_mnemonic_display(mnemonic), timestamp);
        
        // Load the state
        self.load_state(timestamp, mnemonic)?;
        
        // Set the current mnemonic
        self.current_mnemonic = Some(mnemonic.clone());
        
        // Update UI with mnemonic
        let display_name = Self::format_mnemonic_display(mnemonic);
        self.ui.set_session_name(display_name)?;
        
        Ok(())
    }
    
    /// Get the store directory path
    fn get_store_dir() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
        let store_dir = PathBuf::from(home).join(".traxdub").join("store");
        
        // Create directory if it doesn't exist
        if !store_dir.exists() {
            fs::create_dir_all(&store_dir)?;
            info!("Created store directory: {:?}", store_dir);
        }
        
        Ok(store_dir)
    }
    
    /// Generate a random mnemonic (adjective-noun)
    fn generate_mnemonic() -> String {
        use rand::seq::SliceRandom;
        
        let adjectives = [
            "happy", "clever", "brave", "swift", "gentle", "bright", "calm", "cosmic",
            "dancing", "electric", "flowing", "golden", "hidden", "infinite", "jovial",
            "lucid", "mystic", "noble", "peaceful", "quantum", "radiant", "serene",
            "tranquil", "vivid", "wise", "zealous", "ancient", "blazing", "crystal",
            "divine", "emerald", "fierce", "graceful", "harmonic", "mighty", "lunar",
            "sacred", "stellar", "velvet", "wandering", "crimson", "eternal", "frozen",
            "majestic", "silver", "thundering", "sonic", "magnetic", "celestial", "azure",
            "amber", "brilliant", "cosmic", "dreaming", "enigmatic", "fabled", "glowing",
        ];
        
        let nouns = [
            "aurora", "breeze", "cascade", "delta", "echo", "falcon", "galaxy", "harbor",
            "island", "journey", "lagoon", "meadow", "nebula", "oasis", "pulse", "quasar",
            "river", "stream", "tide", "universe", "valley", "wave", "zenith", "beacon",
            "citadel", "desert", "ember", "forest", "glacier", "horizon", "iris",
            "mountain", "ocean", "phoenix", "rain", "sanctuary", "thunder", "vortex",
            "whisper", "crystal", "flame", "shadow", "storm", "comet", "dune",
            "eclipse", "fjord", "grove", "mirage", "nova", "prism", "solstice",
        ];
        
        let mut rng = rand::thread_rng();
        let adjective = adjectives.choose(&mut rng).unwrap();
        let noun = nouns.choose(&mut rng).unwrap();
        
        format!("{}-{}", adjective, noun)
    }
    
    /// Format mnemonic for display (capitalize each word, separate with spaces)
    fn format_mnemonic_display(mnemonic: &str) -> String {
        mnemonic.split('-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    
    /// Get current timestamp in filename format
    fn get_timestamp() -> String {
        Local::now().format("%Y-%m-%d-%H-%M").to_string()
    }
    
    /// Build filename from timestamp and mnemonic
    fn build_filename(timestamp: &str, mnemonic: &str) -> String {
        format!("{}-{}.txd", timestamp, mnemonic)
    }
    
    /// Parse filename to extract timestamp and mnemonic
    fn parse_filename(filename: &str) -> Option<(String, String)> {
        if !filename.ends_with(".txd") {
            return None;
        }
        
        let name = filename.strip_suffix(".txd")?;
        let parts: Vec<&str> = name.splitn(6, '-').collect();
        
        // Format: YYYY-MM-DD-HH-MM-mnemonic-mnemonic
        if parts.len() >= 6 {
            let timestamp = parts[0..5].join("-");
            let mnemonic = parts[5..].join("-");
            Some((timestamp, mnemonic))
        } else {
            None
        }
    }
    
    /// Format timestamp for human-readable display
    fn format_timestamp_display(timestamp: &str) -> String {
        // Parse timestamp: YYYY-MM-DD-HH-MM
        let parts: Vec<&str> = timestamp.split('-').collect();
        if parts.len() != 5 {
            return timestamp.to_string();
        }
        
        let year: i32 = parts[0].parse().unwrap_or(2026);
        let month: u32 = parts[1].parse().unwrap_or(1);
        let day: u32 = parts[2].parse().unwrap_or(1);
        let hour: u32 = parts[3].parse().unwrap_or(0);
        let minute: u32 = parts[4].parse().unwrap_or(0);
        
        if let Some(dt) = chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d| d.and_hms_opt(hour, minute, 0))
            .and_then(|dt: chrono::NaiveDateTime| Local.from_local_datetime(&dt).single())
        {
            let now = Local::now();
            let today = now.date_naive();
            let yesterday = today - chrono::Duration::days(1);
            let file_date = dt.date_naive();
            
            if file_date == today {
                format!("Today - {:02}:{:02}", hour, minute)
            } else if file_date == yesterday {
                format!("Yesterday - {:02}:{:02}", hour, minute)
            } else {
                format!("{} - {:02}:{:02}", dt.format("%B %d"), hour, minute)
            }
        } else {
            timestamp.to_string()
        }
    }
    
    /// Save current engine state
    fn save_state(&mut self) -> Result<()> {
        // Use existing mnemonic or generate new one
        let mnemonic = if let Some(ref m) = self.current_mnemonic {
            m.clone()
        } else {
            let m = Self::generate_mnemonic();
            self.current_mnemonic = Some(m.clone());
            m
        };
        
        let timestamp = Self::get_timestamp();
        let filename = Self::build_filename(&timestamp, &mnemonic);
        let store_dir = Self::get_store_dir()?;
        let filepath = store_dir.join(&filename);
        
        info!("Saving state to: {:?}", filepath);
        
        // Get raw state from engine
        let state_data = self.engine.get_raw_state()?;
        
        // Write to file
        fs::write(&filepath, state_data)?;
        
        // Update UI with mnemonic
        let display_name = Self::format_mnemonic_display(&mnemonic);
        self.ui.set_session_name(display_name)?;
        
        info!("State saved successfully");
        Ok(())
    }
    
    /// Load engine state from file
    fn load_state(&self, timestamp: &str, mnemonic: &str) -> Result<()> {
        let filename = Self::build_filename(timestamp, mnemonic);
        let store_dir = Self::get_store_dir()?;
        let filepath = store_dir.join(&filename);
        
        debug!("Loading state from: {:?}", filepath);
        
        // Read file content
        let state_data = fs::read_to_string(&filepath)?;
        
        // Set engine state
        self.engine.set_raw_state(&state_data)?;
        
        // Get the graph from engine
        let graph = self.engine.get_graph()?;
        
        // Update UI with the graph
        self.load_ui_graph(&graph)?;
        
        // Connect JACK ports for system ports
        self.connect_system_ports(&graph)?;
        
        debug!("State loaded successfully");
        Ok(())
    }
    
    /// Connect JACK ports for system ports in the graph
    fn connect_system_ports(&self, graph: &crate::engine::Graph) -> Result<()> {
        debug!("Connecting system ports to JACK");
        
        // Get all sources and sinks from JACK
        let audio_sources = self.driver.get_sources(PortType::Audio)?;
        let audio_sinks = self.driver.get_sinks(PortType::Audio)?;
        let midi_sources = self.driver.get_sources(PortType::Midi)?;
        let midi_sinks = self.driver.get_sinks(PortType::Midi)?;
        
        // Process each system port in the graph
        for port in &graph.ports {
            // port.id is already sanitized, just extract the last segment
            let sanitized_name = port.id.split('/').last().unwrap_or(&port.id);
            
            // Determine the port type for filtering
            let driver_port_type = match port.port_type {
                crate::engine::PortType::Audio => PortType::Audio,
                crate::engine::PortType::Midi => PortType::Midi,
            };
            
            // Match based on direction and type
            match (&port.direction, driver_port_type) {
                (crate::engine::PortDirection::Input, PortType::Audio) => {
                    // Find matching audio source
                    self.find_and_connect_source(&audio_sources, &sanitized_name, &port.id)?;
                }
                (crate::engine::PortDirection::Input, PortType::Midi) => {
                    // Find matching MIDI source
                    self.find_and_connect_source(&midi_sources, &sanitized_name, &port.id)?;
                }
                (crate::engine::PortDirection::Output, PortType::Audio) => {
                    // Find matching audio sink
                    self.find_and_connect_sink(&audio_sinks, &sanitized_name, &port.id)?;
                }
                (crate::engine::PortDirection::Output, PortType::Midi) => {
                    // Find matching MIDI sink
                    self.find_and_connect_sink(&midi_sinks, &sanitized_name, &port.id)?;
                }
                _ => {
                    // PortType::All should not occur when converting from engine::PortType
                    warn!("Unexpected port type combination for system port: {}", port.id);
                }
            }
        }
        
        Ok(())
    }
    
    /// Find and connect a source port
    fn find_and_connect_source(&self, sources: &[crate::controller::driver::Source], sanitized_name: &str, system_port_id: &str) -> Result<()> {
        for source in sources {
            for port in &source.ports {
                let port_sanitized = Driver::sanitize_port_name(&port.name);
                if port_sanitized == sanitized_name {
                    debug!("Connecting source {} to ingen system port {}", port.name, system_port_id);
                    
                    // Build the ingen port name with TraxDub prefix
                    let ingen_port_name = format!("TraxDub Engine:{}", sanitized_name);
                    
                    // Create destination port struct
                    let dest_port = crate::controller::driver::Port {
                        name: ingen_port_name,
                        short_name: sanitized_name.to_string(),
                    };
                    
                    if let Err(e) = self.driver.connect_ports(port, &dest_port) {
                        warn!("Failed to connect {} to {}: {}", port.name, system_port_id, e);
                    }
                    return Ok(());
                }
            }
        }
        debug!("No matching JACK source found for system port: {}", system_port_id);
        Ok(())
    }
    
    /// Find and connect a sink port
    fn find_and_connect_sink(&self, sinks: &[crate::controller::driver::Sink], sanitized_name: &str, system_port_id: &str) -> Result<()> {
        for sink in sinks {
            for port in &sink.ports {
                let port_sanitized = Driver::sanitize_port_name(&port.name);
                if port_sanitized == sanitized_name {
                    debug!("Connecting ingen system port {} to sink {}", system_port_id, port.name);
                    
                    // Build the ingen port name with TraxDub prefix
                    let ingen_port_name = format!("TraxDub Engine:{}", sanitized_name);
                    
                    // Create source port struct
                    let source_port = crate::controller::driver::Port {
                        name: ingen_port_name,
                        short_name: sanitized_name.to_string(),
                    };
                    
                    if let Err(e) = self.driver.connect_ports(&source_port, port) {
                        warn!("Failed to connect {} to {}: {}", system_port_id, port.name, e);
                    }
                    return Ok(());
                }
            }
        }
        debug!("No matching JACK sink found for system port: {}", system_port_id);
        Ok(())
    }
    
    /// Load UI graph from engine graph data
    fn load_ui_graph(&self, graph: &crate::engine::Graph) -> Result<()> {
        debug!("Loading UI graph from engine data");
        
        // Clear existing UI nodes and links (except system nodes)
        // Note: We should add a method to clear the UI, but for now we'll just add nodes
        
        // Create nodes for each block
        for block in &graph.blocks {
        
          debug!("Creating UI node: {} ", block.name);
            self.ui.create_node(
                block.id.clone(),
                block.name.clone(),
                crate::ui::NodeType::Normal,
            )?;
        }
        
        // Create nodes for each system port
        for port in &graph.ports {
            debug!("Creating UI node for system port: {}", port.id);
            
            let port_node_id = format!("ingen:/main/{}", port.id);
            
            let port_type = match port.direction {
                crate::engine::PortDirection::Input => crate::ui::NodeType::PortIn,
                crate::engine::PortDirection::Output => crate::ui::NodeType::PortOut,
            };

            self.ui.create_node(
                port_node_id.clone(),
                port.id.clone(),
                port_type
            )?;
            
            // Link system port to its corresponding system node
            match port.direction {
                crate::engine::PortDirection::Input => {
                    // Input ports connect from "inputs" to the port (Context -> PortIn => PortIn)
                    if let Err(e) = self.ui.create_link("inputs".to_string(), port_node_id, crate::ui::LinkType::PortIn) {
                        debug!("Failed to create link from inputs to {}: {}", port.id, e);
                    }
                }
                crate::engine::PortDirection::Output => {
                    // Output ports connect from the port to "outputs" (PortOut -> Context => PortOut)
                    if let Err(e) = self.ui.create_link(port_node_id, "outputs".to_string(), crate::ui::LinkType::PortOut) {
                        debug!("Failed to create link from {} to outputs: {}", port.id, e);
                    }
                }
            }
        }
        
        // Create links for each connection
        for connection in &graph.connections {
            // Extract node IDs from port paths
            // Port path format: "ingen:/main/node_id/port_id"
            let from_id = self.extract_node_from_port(&connection.source);
            let to_id = self.extract_node_from_port(&connection.destination);
            
            debug!("Creating UI link: {} -> {}", from_id, to_id);
            
            // Only create link if we haven't already
            // Note: create_link should handle duplicates gracefully
            // For connections between blocks, use Normal link type
            if let Err(e) = self.ui.create_link(from_id, to_id, crate::ui::LinkType::Normal) {
                debug!("Link creation failed (may already exist): {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Extract node ID from a port path
    fn extract_node_from_port(&self, port_path: &str) -> String {
        // Port path format: "ingen:/main/node_id/port_id" or "ingen:/main/system_port"
        // We want to extract the first three segments: "ingen:/main/node_id"
        let parts: Vec<&str> = port_path.split('/').collect();
        if parts.len() >= 3 {
            parts[..3].join("/")
        } else {
            port_path.to_string()
        }
    }
    
    /// Get list of all saved mnemonics (newest first)
    fn get_saved_mnemonics() -> Result<Vec<String>> {
        let store_dir = Self::get_store_dir()?;
        
        if !store_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut mnemonics: Vec<(String, String)> = Vec::new();
        
        for entry in fs::read_dir(store_dir)? {
            let entry = entry?;
            let filename = entry.file_name().to_string_lossy().to_string();
            
            if let Some((timestamp, mnemonic)) = Self::parse_filename(&filename) {
                mnemonics.push((timestamp, mnemonic));
            }
        }
        
        // Sort by timestamp descending
        mnemonics.sort_by(|a, b| b.0.cmp(&a.0));
        
        // Extract unique mnemonics in order
        let mut unique_mnemonics = Vec::new();
        let mut seen = std::collections::HashSet::new();
        
        for (_, mnemonic) in mnemonics {
            if seen.insert(mnemonic.clone()) {
                unique_mnemonics.push(mnemonic);
            }
        }
        
        Ok(unique_mnemonics)
    }
    
    /// Get all timestamps for a given mnemonic (newest first)
    fn get_mnemonic_timestamps(mnemonic: &str) -> Result<Vec<String>> {
        let store_dir = Self::get_store_dir()?;
        
        let mut timestamps = Vec::new();
        
        for entry in fs::read_dir(store_dir)? {
            let entry = entry?;
            let filename = entry.file_name().to_string_lossy().to_string();
            
            if let Some((timestamp, file_mnemonic)) = Self::parse_filename(&filename) {
                if file_mnemonic == mnemonic {
                    timestamps.push(timestamp);
                }
            }
        }
        
        // Sort by timestamp descending
        timestamps.sort_by(|a, b| b.cmp(a));
        
        Ok(timestamps)
    }
    
    /// Get the file menu
    fn get_file_menu(&self) -> Menu {
        Menu {
            id: "file_menu".to_string(),
            name: "File".to_string(),
            options: vec![
                MenuOption {
                    id: "save".to_string(),
                    name: "Save".to_string(),
                },
                MenuOption {
                    id: "load".to_string(),
                    name: "Load...".to_string(),
                },
            ],
        }
    }
    
    /// Get the load selection menu (list of mnemonics)
    fn get_load_selection_menu(&self) -> Menu {
        let mnemonics = Self::get_saved_mnemonics().unwrap_or_default();
        
        let options: Vec<MenuOption> = mnemonics.iter()
            .map(|mnemonic| MenuOption {
                id: mnemonic.clone(),
                name: Self::format_mnemonic_display(mnemonic),
            })
            .collect();
        
        Menu {
            id: "load_selection".to_string(),
            name: "Select Session".to_string(),
            options,
        }
    }
    
    /// Get the timestamp selection menu for a mnemonic
    fn get_timestamp_selection_menu(&self, mnemonic: &str) -> Menu {
        let timestamps = Self::get_mnemonic_timestamps(mnemonic).unwrap_or_default();
        
        let options: Vec<MenuOption> = timestamps.iter()
            .map(|timestamp| MenuOption {
                id: timestamp.clone(),
                name: Self::format_timestamp_display(timestamp),
            })
            .collect();
        
        Menu {
            id: format!("timestamp_selection_{}", mnemonic),
            name: Self::format_mnemonic_display(mnemonic),
            options,
        }
    }
}

impl Feature for PersistenceFeature {
    fn get_menu(&self) -> Menu {
        match &self.menu_state {
            PersistenceMenuState::FileMenu => self.get_file_menu(),
            PersistenceMenuState::LoadSelection => self.get_load_selection_menu(),
            PersistenceMenuState::TimestampSelection(mnemonic) => {
                self.get_timestamp_selection_menu(mnemonic)
            }
        }
    }
    
    fn handle_menu_option(&mut self, option_id: Option<&str>, _element: Option<&crate::ui::Element>) -> Result<ControllerState> {
        debug!("Persistence feature handle_menu_option: {:?}", option_id);
        
        let Some(option) = option_id else {
            debug!("Persistence feature: menu closed");
            self.menu_state = PersistenceMenuState::FileMenu;
            return Ok(ControllerState::Navigating);
        };
        
        match &self.menu_state {
            PersistenceMenuState::FileMenu => {
                match option {
                    "save" => {
                        self.save_state()?;
                        self.menu_state = PersistenceMenuState::FileMenu;
                        Ok(ControllerState::Navigating)
                    }
                    "load" => {
                        self.menu_state = PersistenceMenuState::LoadSelection;
                        Ok(ControllerState::BrowsingMenu)
                    }
                    _ => Ok(ControllerState::Navigating),
                }
            }
            PersistenceMenuState::LoadSelection => {
                // Mnemonic selected, show timestamps
                self.menu_state = PersistenceMenuState::TimestampSelection(option.to_string());
                Ok(ControllerState::BrowsingMenu)
            }
            PersistenceMenuState::TimestampSelection(mnemonic) => {
                // Timestamp selected, load the file
                self.load_state(option, mnemonic)?;
                self.menu_state = PersistenceMenuState::FileMenu;
                Ok(ControllerState::Navigating)
            }
        }
    }
}

/// Helper to create a new persistence feature
pub fn new_persistence_feature(driver: Arc<Driver>, engine: Arc<Engine>, ui: Arc<UI>, auto_load: bool) -> PersistenceFeature {
    PersistenceFeature::new(driver, engine, ui, auto_load)
}

use anyhow::{anyhow, Result};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Represents a plugin in the Ingen graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub uri: String,
    pub name: String,
    pub label: String,
}

/// Represents a block (plugin instance) in the Ingen graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub plugin_uri: String,
    pub path: String,
}

/// Represents a connection between two ports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub source: String,
    pub destination: String,
}

/// Control parameter for a block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlParameter {
    pub block_id: String,
    pub parameter_name: String,
    pub value: f32,
}

/// Engine module that encapsulates an Ingen instance
pub struct Engine {
    blocks: HashMap<String, Block>,
    connections: Vec<Connection>,
    available_plugins: Vec<Plugin>,
    ingen_process: Option<std::process::Child>,
}

impl Engine {
    /// Create a new engine instance
    pub fn new() -> Result<Self> {
        info!("Initializing Engine...");

        let mut engine = Self {
            blocks: HashMap::new(),
            connections: Vec::new(),
            available_plugins: Vec::new(),
            ingen_process: None,
        };

        // Start Ingen in the background
        engine.start_ingen()?;

        // Discover available plugins
        engine.discover_plugins()?;

        Ok(engine)
    }

    /// Start the Ingen process
    fn start_ingen(&mut self) -> Result<()> {
        info!("Starting Ingen process...");

        use std::process::{Command, Stdio};
        use std::thread;
        use std::time::Duration;

        let child = Command::new("ingen")
            .arg("-e")  // Engine mode
            .arg("-S")  // Socket path
            .arg("/tmp/ingen-traxdub.sock")
            .arg("-n")  // Client name
            .arg("TraxDub Engine")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to start ingen process: {}. Make sure ingen is installed.", e))?;

        info!("Ingen process started (PID: {:?})", child.id());
        self.ingen_process = Some(child);

        // Give Ingen time to initialize and create the socket
        info!("Waiting for Ingen to initialize...");
        thread::sleep(Duration::from_millis(500));

        Ok(())
    }

    /// Discover available LV2 plugins
    fn discover_plugins(&mut self) -> Result<()> {
        info!("Discovering available plugins...");

        // In a real implementation, you would query LV2 plugins
        // For now, we'll add some placeholder plugins
        self.available_plugins = vec![
            Plugin {
                uri: "http://lv2plug.in/plugins/eg-amp".to_string(),
                name: "Amplifier".to_string(),
                label: "Amp".to_string(),
            },
            Plugin {
                uri: "http://lv2plug.in/plugins/eg-reverb".to_string(),
                name: "Reverb".to_string(),
                label: "Reverb".to_string(),
            },
        ];

        info!("Discovered {} plugins", self.available_plugins.len());
        Ok(())
    }

    /// List all available plugins
    pub fn list_available_plugins(&self) -> Vec<Plugin> {
        debug!("Listing {} available plugins", self.available_plugins.len());
        self.available_plugins.clone()
    }

    /// Create a new block (plugin instance)
    pub fn create_block(&mut self, plugin_uri: &str, block_id: &str) -> Result<Block> {
        info!("Creating block '{}' with plugin '{}'", block_id, plugin_uri);

        // Check if plugin exists
        if !self.available_plugins.iter().any(|p| p.uri == plugin_uri) {
            return Err(anyhow!("Plugin not found: {}", plugin_uri));
        }

        // Check if block ID already exists
        if self.blocks.contains_key(block_id) {
            return Err(anyhow!("Block ID already exists: {}", block_id));
        }

        let block = Block {
            id: block_id.to_string(),
            plugin_uri: plugin_uri.to_string(),
            path: format!("/main/{}", block_id),
        };

        // In a real implementation, you would send a command to Ingen
        // For now, we'll just store it locally
        self.blocks.insert(block_id.to_string(), block.clone());

        info!("Block '{}' created successfully", block_id);
        Ok(block)
    }

    /// Get a block by ID
    pub fn get_block(&self, block_id: &str) -> Option<&Block> {
        debug!("Getting block '{}'", block_id);
        self.blocks.get(block_id)
    }

    /// Duplicate a plugin instance
    pub fn duplicate_block(&mut self, source_block_id: &str, new_block_id: &str) -> Result<Block> {
        info!(
            "Duplicating block '{}' to '{}'",
            source_block_id, new_block_id
        );

        // Get the source block
        let source_block = self
            .get_block(source_block_id)
            .ok_or_else(|| anyhow!("Source block not found: {}", source_block_id))?
            .clone();

        // Create a new block with the same plugin
        self.create_block(&source_block.plugin_uri, new_block_id)
    }

    /// Set a control parameter on a block
    pub fn set_control_parameter(
        &mut self,
        block_id: &str,
        parameter_name: &str,
        value: f32,
    ) -> Result<()> {
        info!(
            "Setting parameter '{}' on block '{}' to {}",
            parameter_name, block_id, value
        );

        // Check if block exists
        if !self.blocks.contains_key(block_id) {
            return Err(anyhow!("Block not found: {}", block_id));
        }

        // In a real implementation, you would send a command to Ingen
        // For now, we'll just log it
        debug!(
            "Parameter set: {}.{} = {}",
            block_id, parameter_name, value
        );

        Ok(())
    }

    /// Connect two ports
    pub fn connect(&mut self, source: &str, destination: &str) -> Result<()> {
        info!("Connecting '{}' to '{}'", source, destination);

        // In a real implementation, you would validate the ports exist
        // and send a command to Ingen

        let connection = Connection {
            source: source.to_string(),
            destination: destination.to_string(),
        };

        self.connections.push(connection);
        info!("Connection created successfully");

        Ok(())
    }

    /// Disconnect two ports
    pub fn disconnect(&mut self, source: &str, destination: &str) -> Result<()> {
        info!("Disconnecting '{}' from '{}'", source, destination);

        // Remove the connection
        self.connections
            .retain(|c| !(c.source == source && c.destination == destination));

        info!("Connection removed successfully");
        Ok(())
    }

    /// Load an Ingen graph from a file
    pub fn load(&mut self, path: &Path) -> Result<()> {
        info!("Loading graph from {:?}", path);

        if !path.exists() {
            return Err(anyhow!("Graph file not found: {:?}", path));
        }

        // In a real implementation, you would:
        // 1. Send a command to Ingen to load the graph
        // 2. Update internal state from the loaded graph
        
        // For now, we'll just log it
        info!("Graph loaded successfully (placeholder)");

        Ok(())
    }

    /// Save the current Ingen graph to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        info!("Saving graph to {:?}", path);

        // In a real implementation, you would:
        // 1. Send a command to Ingen to save the graph
        // 2. Serialize the current state
        
        // For now, we'll create a simple JSON representation
        let graph_data = serde_json::json!({
            "blocks": self.blocks.values().collect::<Vec<_>>(),
            "connections": &self.connections,
        });

        std::fs::write(path, serde_json::to_string_pretty(&graph_data)?)?;
        info!("Graph saved successfully");

        Ok(())
    }

    /// Get all blocks
    pub fn get_all_blocks(&self) -> Vec<&Block> {
        self.blocks.values().collect()
    }

    /// Get all connections
    pub fn get_all_connections(&self) -> &[Connection] {
        &self.connections
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        info!("Shutting down Engine...");
        
        // Clean up Ingen process if running
        if let Some(mut process) = self.ingen_process.take() {
            let _ = process.kill();
            info!("Ingen process terminated");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_block() {
        let mut engine = Engine::new().unwrap();
        let block = engine
            .create_block("http://lv2plug.in/plugins/eg-amp", "amp1")
            .unwrap();

        assert_eq!(block.id, "amp1");
        assert_eq!(block.plugin_uri, "http://lv2plug.in/plugins/eg-amp");
    }

    #[test]
    fn test_duplicate_block() {
        let mut engine = Engine::new().unwrap();
        engine
            .create_block("http://lv2plug.in/plugins/eg-amp", "amp1")
            .unwrap();

        let duplicate = engine.duplicate_block("amp1", "amp2").unwrap();
        assert_eq!(duplicate.id, "amp2");
        assert_eq!(duplicate.plugin_uri, "http://lv2plug.in/plugins/eg-amp");
    }

    #[test]
    fn test_connect_disconnect() {
        let mut engine = Engine::new().unwrap();
        engine
            .connect("/main/source:output", "/main/dest:input")
            .unwrap();

        assert_eq!(engine.connections.len(), 1);

        engine
            .disconnect("/main/source:output", "/main/dest:input")
            .unwrap();

        assert_eq!(engine.connections.len(), 0);
    }
}

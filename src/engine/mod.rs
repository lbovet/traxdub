pub mod protocol;
pub mod lv2;

use anyhow::{anyhow, Result};
use log::{debug, info, warn, trace};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use protocol::IngenProtocol;

/// Port type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortType {
    Audio,
    Midi,
}

/// Port direction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortDirection {
    Input,
    Output,
}

/// Port information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Port {
    /// Port identifier/name
    pub id: String,
    /// Port type (Audio or Midi)
    pub port_type: PortType,
    /// Port direction (Input or Output)
    pub direction: PortDirection,
}

/// Plugin metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plugin {
    /// The plugin IRI/URI
    pub id: String,
    /// The plugin name
    pub name: String,
    /// List of ports
    pub ports: Vec<Port>,
}

/// Engine module that encapsulates an Ingen instance
pub struct Engine {
    ingen_process: Option<std::process::Child>,
    socket: Mutex<Option<UnixStream>>,
    /// List of available LV2 plugins
    plugins: Vec<Plugin>,
}

impl Engine {
    /// Create a new engine instance
    /// 
    /// # Arguments
    /// * `use_external` - If true, connect to an external Ingen instance instead of starting a new one
    pub fn new(use_external: bool) -> Result<Self> {
        debug!("Initializing Engine...");

        let mut engine = Self {
            ingen_process: None,
            socket: Mutex::new(None),
            plugins: Vec::new(),
        };

        // Start Ingen in the background (unless using external)
        if !use_external {
            engine.start_ingen()?;
        } else {
            info!("Using external Ingen instance");
        }
        
        // Connect to Ingen socket
        engine.connect_socket()?;

        // Discover available plugins from Ingen
        let ingen_plugin_iris = engine.discover_plugins()?;
        
        // Get full plugin metadata from LV2
        let lv2_world = lv2::Lv2World::new()?;
        let all_lv2_plugins = lv2_world.list_plugins();
        
        // Filter to keep only plugins that Ingen knows about
        engine.plugins = all_lv2_plugins.into_iter()
            .filter(|plugin| ingen_plugin_iris.contains(&plugin.id))
            .collect();
        
        info!("Found {} plugins", engine.plugins.len());        

        trace!("Available plugins: {:?}", engine.plugins);

        Ok(engine)
    }

    /// Start the Ingen process
    fn start_ingen(&mut self) -> Result<()> {
        debug!("Starting Ingen process...");

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
        debug!("Waiting for Ingen to initialize...");
        thread::sleep(Duration::from_millis(500));

        Ok(())
    }

    /// Connect to the Ingen Unix socket
    fn connect_socket(&mut self) -> Result<()> {
        debug!("Connecting to Ingen socket...");
        
        let socket_path = "/tmp/ingen-traxdub.sock";
        
        // Retry connection a few times in case Ingen is still initializing
        let mut attempts = 0;
        let max_attempts = 10;
        
        loop {
            match UnixStream::connect(socket_path) {
                Ok(stream) => {
                    info!("Connected to Ingen socket");
                    *self.socket.lock().unwrap() = Some(stream);
                    
                    // Send initialization message with RDF prefixes
                    debug!("Sending initialization message to Ingen");
                    self.send_message(IngenProtocol::get_init_message())?;
                    
                    return Ok(());
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(anyhow!("Failed to connect to Ingen socket after {} attempts: {}", max_attempts, e));
                    }
                    warn!("Socket connection attempt {} failed, retrying...", attempts);
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }
    }
    
    /// Send a message to Ingen via the Unix socket
    fn send_message(&self, message: &str) -> Result<()> {
        debug!("Sending message to Ingen: {}", message);
        
        let mut socket_guard = self.socket.lock().unwrap();
        if let Some(socket) = socket_guard.as_mut() {
            socket.write_all(message.as_bytes())
                .map_err(|e| anyhow!("Failed to write to Ingen socket: {}", e))?;
            socket.flush()
                .map_err(|e| anyhow!("Failed to flush Ingen socket: {}", e))?;
            Ok(())
        } else {
            Err(anyhow!("Not connected to Ingen socket"))
        }
    }

    /// Receive a message from Ingen via Unix socket
    fn receive_message(&self) -> Result<String> {
        use std::io::Read;
        
        loop {
            debug!("Receiving message from Ingen...");
            
            let mut socket_guard = self.socket.lock().unwrap();
            if let Some(socket) = socket_guard.as_mut() {
                // Set a read timeout
                socket.set_read_timeout(Some(Duration::from_secs(5)))
                    .map_err(|e| anyhow!("Failed to set read timeout: {}", e))?;
                
                let mut buffer = String::new();
                let mut temp_buf = [0u8; 4096];
                
                loop {
                    match socket.read(&mut temp_buf) {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            // Check if null byte is in the received data
                            if let Some(null_pos) = temp_buf[..n].iter().position(|&b| b == 0) {
                                // Add data up to (but not including) the null byte
                                buffer.push_str(&String::from_utf8_lossy(&temp_buf[..null_pos]));
                                break;
                            }
                            buffer.push_str(&String::from_utf8_lossy(&temp_buf[..n]));
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(e) => return Err(anyhow!("Failed to read from Ingen socket: {}", e)),
                    }
                }
                
                debug!("Received {} bytes from Ingen", buffer.len());
                trace!("Received buffer content:\n{}", buffer);
                
                // Drop the socket guard before checking bundle boundary
                drop(socket_guard);
                
                // Check if this is a bundle boundary message - if so, ignore and receive again
                if IngenProtocol::is_bundle_boundary(&buffer) {
                    debug!("Received bundle boundary, ignoring and receiving next message");
                    continue;
                }
                
                return Ok(buffer);
            } else {
                return Err(anyhow!("Not connected to Ingen socket"));
            }
        }
    }

    /// Discover available LV2 plugins from Ingen
    fn discover_plugins(&mut self) -> Result<Vec<String>> {
        debug!("Discovering LV2 plugins from Ingen...");
        
        self.send_message(&IngenProtocol::build_get_plugins()?)?;
        let plugins = 
            IngenProtocol::parse_get_plugins(&self.receive_message()?)?;
        
        debug!("Ingen reported {} plugins", plugins.len());
        Ok(plugins)
    }

    /// Get the list of available plugins
    pub fn list_plugins(&self) -> &[Plugin] {
        &self.plugins
    }

    /// Create a new block (plugin instance)
    pub fn create_block(&self, plugin_uri: &str, block_id: &str) -> Result<()> {
        info!("Creating block '{}' with plugin '{}'", block_id, plugin_uri);
        
        // Build RDF message using protocol module
        let message = IngenProtocol::build_create_block(block_id, plugin_uri)?;
        
        // Send to Ingen
        self.send_message(&message)?;
        self.receive_message()?; // Drain response
        
        Ok(())
    }

    /// Duplicate a plugin instance
    pub fn duplicate_block(&self, source_block_id: &str, new_block_id: &str) -> Result<()> {
        // TODO: Implement
        Ok(())
    }

    /// Set a control parameter on a block
    pub fn set_control_parameter(
        &self,
        block_id: &str,
        parameter_name: &str,
        value: f32,
    ) -> Result<()> {
        // TODO: Implement
        Ok(())
    }

    /// Connect two ports
    pub fn connect(&self, source: &str, destination: &str) -> Result<()> {
        info!("Connecting '{}' to '{}'", source, destination);

        // Build RDF message using protocol module
        let message = IngenProtocol::build_connect(source, destination)?;
        
        // Send to Ingen
        self.send_message(&message)?;
        self.receive_message()?; // Drain response

        Ok(())
    }

    /// Disconnect two ports
    pub fn disconnect(&self, source: &str, destination: &str) -> Result<()> {
        info!("Disconnecting '{}' from '{}'", source, destination);

        // Build RDF message using protocol module
        let message = IngenProtocol::build_disconnect(source, destination)?;
        
        // Send to Ingen
        self.send_message(&message)?;
        self.receive_message()?; // Drain response

        Ok(())
    }

    /// Create an input port
    pub fn create_input_port(&self, port_name: &str, port_type: PortType) -> Result<String> {
        info!("Creating {:?} input port '{}'", port_type, port_name);

        // Build RDF message using protocol module
        let message = IngenProtocol::build_create_port(port_name, &port_type, &PortDirection::Input)?;
        
        // Send to Ingen
        self.send_message(&message)?;
        self.receive_message()?; // Drain response

        // Return the port path
        Ok(format!("ingen:/main/{}", port_name))
    }

    /// Create an output port
    pub fn create_output_port(&self, port_name: &str, port_type: PortType) -> Result<String> {
        info!("Creating {:?} output port '{}'", port_type, port_name);

        // Build RDF message using protocol module
        let message = IngenProtocol::build_create_port(port_name, &port_type, &PortDirection::Output)?;
        
        // Send to Ingen
        self.send_message(&message)?;
        self.receive_message()?; // Drain response

        // Return the port path
        Ok(format!("ingen:/main/{}", port_name))
    }

    /// Get the raw state of the engine as a string
    pub fn get_raw_state(&self) -> Result<String> {
        info!("Getting raw engine state");
        
        // Build RDF message using protocol module
        let message = IngenProtocol::build_get_state()?;
        
        // Send to Ingen
        self.send_message(&message)?;
        
        // Receive response (full state)
        let response = self.receive_message()?;
        
        Ok(response)
    }

    /// Set the raw state of the engine from a string
    pub fn set_raw_state(&self, state_data: &str) -> Result<()> {
        info!("Setting raw engine state ({} bytes)", state_data.len());
        
        // Send data diretly to Ingen
        self.send_message(state_data)?;
        self.receive_message()?; // Drain response
        
        Ok(())
    }

}

impl Drop for Engine {
    fn drop(&mut self) {
        debug!("Shutting down Engine...");
        
        // Clean up Ingen process if running
        if let Some(mut process) = self.ingen_process.take() {
            match process.kill() {
                Ok(_) => {
                    // Wait for the process to actually exit
                    match process.wait() {
                        Ok(status) => debug!("Ingen process exited with {}", status),
                        Err(e) => eprintln!("Error waiting for Ingen process to exit: {}", e),
                    }
                }
                Err(e) => {
                    eprintln!("Error killing Ingen process: {}", e);
                }
            }
        }
    }
}
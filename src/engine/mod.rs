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

/// Block in the graph (plugin instance)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// Block path/ID (e.g., "ingen:/main/block_id")
    pub id: String,
    /// Block name
    pub name: String,
    /// List of ports
    pub ports: Vec<Port>,
}

/// Connection between two ports
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    /// Source port path (e.g., "ingen:/main/block1/out")
    pub source: String,
    /// Destination port path (e.g., "ingen:/main/block2/in")
    pub destination: String,
}

/// Graph representation of the current Ingen state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Graph {
    /// List of blocks in the graph
    pub blocks: Vec<Block>,
    /// List of connections between ports
    pub connections: Vec<Connection>,
    /// List of system ports (ports directly under ingen:/main/)
    pub ports: Vec<Port>,
}

/// Engine module that encapsulates an Ingen instance
pub struct Engine {
    ingen_process: Option<std::process::Child>,
    socket: Mutex<Option<UnixStream>>,
    /// List of available LV2 plugins
    plugins: Vec<Plugin>,
    /// Buffer for leftover bytes after null terminator
    read_buffer: Mutex<Vec<u8>>,
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
            read_buffer: Mutex::new(Vec::new()),
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
    
    /// Drain any pending response data from the socket
    fn drain_response(&self) -> Result<()> {
        use std::io::Read;
        
        let mut socket_guard = self.socket.lock().unwrap();
        if let Some(socket) = socket_guard.as_mut() {
            // Set non-blocking mode with minimal timeout
            socket.set_read_timeout(Some(Duration::from_millis(1)))
                .map_err(|e| anyhow!("Failed to set read timeout: {}", e))?;
            
            let mut drain_buf = [0u8; 4096];
            let mut total_drained = 0;
            
            // Keep reading and discarding bytes until none are available
            loop {
                match socket.read(&mut drain_buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        total_drained += n;
                        // Continue draining
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || 
                              e.kind() == std::io::ErrorKind::TimedOut => {
                        // No more data available
                        break;
                    }
                    Err(e) => return Err(anyhow!("Failed to drain socket: {}", e)),
                }
            }
            
            if total_drained > 0 {
                debug!("Drained {} bytes from response stream", total_drained);
            }
            
            // Clear the read buffer as well
            self.read_buffer.lock().unwrap().clear();
            
            Ok(())
        } else {
            Err(anyhow!("Not connected to Ingen socket"))
        }
    }
    
    /// Send a message to Ingen via the Unix socket
    fn send_message(&self, message: &str) -> Result<()> {
        debug!("Sending message to Ingen: {}", message);
        
        // Drain any pending responses before sending new message
        self.drain_response()?;

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
            
            // Get any buffered bytes from the previous read
            let mut read_buffer_guard = self.read_buffer.lock().unwrap();
            let buffered = read_buffer_guard.clone();
            read_buffer_guard.clear();
            drop(read_buffer_guard);
            
            let mut buffer = buffered;
            let mut bundle_start_seq: Option<String> = None;
            let mut bundle_end_seq: Option<String> = None;
            let mut found_bundle_end = false;
            
            let mut socket_guard = self.socket.lock().unwrap();
            if let Some(socket) = socket_guard.as_mut() {
                // Set a longer read timeout to handle large messages
                socket.set_read_timeout(Some(Duration::from_secs(30)))
                    .map_err(|e| anyhow!("Failed to set read timeout: {}", e))?;
                
                let mut temp_buf = [0u8; 4096];
                
                // Keep reading until we find the complete bundle (BundleStart -> BundleEnd -> ".")
                loop {
                    match socket.read(&mut temp_buf) {
                        Ok(0) => {
                            // EOF - connection closed
                            if buffer.is_empty() {
                                return Err(anyhow!("Connection closed by Ingen"));
                            }
                            break;
                        }
                        Ok(n) => {
                            // Filter out null bytes and append to buffer
                            for &byte in &temp_buf[..n] {
                                if byte != 0 {
                                    buffer.push(byte);
                                }
                            }
                            
                            // Convert current buffer to string for line parsing
                            let buffer_str = String::from_utf8_lossy(&buffer);
                            
                            // Look for BundleStart sequence number if not found yet
                            if bundle_start_seq.is_none() {
                                if let Some(start_pos) = buffer_str.find("a ingen:BundleStart") {
                                    // Find the next line with patch:sequenceNumber
                                    if let Some(seq_pos) = buffer_str[start_pos..].find("patch:sequenceNumber") {
                                        let after_seq = &buffer_str[start_pos + seq_pos + 20..]; // Skip "patch:sequenceNumber"
                                        // Extract the number in quotes
                                        if let Some(quote_start) = after_seq.find('"') {
                                            if let Some(quote_end) = after_seq[quote_start + 1..].find('"') {
                                                let seq_num = &after_seq[quote_start + 1..quote_start + 1 + quote_end];
                                                debug!("Bundle start sequence: {}", seq_num);
                                                bundle_start_seq = Some(seq_num.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Look for BundleEnd sequence number if start found but end not yet
                            if bundle_start_seq.is_some() && bundle_end_seq.is_none() {
                                if let Some(end_pos) = buffer_str.find("a ingen:BundleEnd") {
                                    // Find the next line with patch:sequenceNumber
                                    if let Some(seq_pos) = buffer_str[end_pos..].find("patch:sequenceNumber") {
                                        let after_seq = &buffer_str[end_pos + seq_pos + 20..]; // Skip "patch:sequenceNumber"
                                        // Extract the number in quotes
                                        if let Some(quote_start) = after_seq.find('"') {
                                            if let Some(quote_end) = after_seq[quote_start + 1..].find('"') {
                                                let seq_num = &after_seq[quote_start + 1..quote_start + 1 + quote_end];
                                                debug!("Bundle end sequence: {}", seq_num);
                                                bundle_end_seq = Some(seq_num.to_string());
                                                found_bundle_end = true;
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // If we found bundle end, look for the final dot
                            if found_bundle_end {
                                if let Some(dot_pos) = buffer_str.rfind('.') {
                                    // Check if this dot is the last non-whitespace character
                                    let after_dot = &buffer_str[dot_pos + 1..];
                                    if after_dot.trim().is_empty() {
                                        // Found the response boundary
                                        debug!("Found response boundary at dot position {}", dot_pos);
                                        
                                        // Check if there are more bytes available to read
                                        socket.set_read_timeout(Some(Duration::from_millis(1)))
                                            .map_err(|e| anyhow!("Failed to set read timeout: {}", e))?;
                                        
                                        let mut peek_buf = [0u8; 1];
                                        match socket.read(&mut peek_buf) {
                                            Ok(0) => {
                                                // No more data, we can stop
                                                // Save any bytes after the dot for next read
                                                let dot_byte_pos = buffer_str[..=dot_pos].len();
                                                if buffer.len() > dot_byte_pos {
                                                    let remaining = &buffer[dot_byte_pos..];
                                                    let mut read_buffer_guard = self.read_buffer.lock().unwrap();
                                                    read_buffer_guard.extend_from_slice(remaining);
                                                }
                                                
                                                // Truncate buffer at the dot (inclusive)
                                                buffer.truncate(dot_byte_pos);
                                                break;
                                            }
                                            Ok(_) => {
                                                // More data available, put the peeked byte back into buffer
                                                if peek_buf[0] != 0 {
                                                    buffer.push(peek_buf[0]);
                                                }
                                                // Reset timeout and continue reading
                                                socket.set_read_timeout(Some(Duration::from_secs(30)))
                                                    .map_err(|e| anyhow!("Failed to set read timeout: {}", e))?;
                                                // Continue to next read
                                            }
                                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                                                // Timeout means no more data available
                                                // Save any bytes after the dot for next read
                                                let dot_byte_pos = buffer_str[..=dot_pos].len();
                                                if buffer.len() > dot_byte_pos {
                                                    let remaining = &buffer[dot_byte_pos..];
                                                    let mut read_buffer_guard = self.read_buffer.lock().unwrap();
                                                    read_buffer_guard.extend_from_slice(remaining);
                                                }
                                                
                                                // Truncate buffer at the dot (inclusive)
                                                buffer.truncate(dot_byte_pos);
                                                break;
                                            }
                                            Err(e) => return Err(anyhow!("Failed to peek socket: {}", e)),
                                        }
                                    }

                                }
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // Timeout
                            if !buffer.is_empty() {
                                warn!("Read timeout after receiving {} bytes", buffer.len());
                                break;
                            } else {
                                return Err(anyhow!("Timeout waiting for data from Ingen"));
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                            // Interrupted system call, retry
                            continue;
                        }
                        Err(e) => return Err(anyhow!("Failed to read from Ingen socket: {}", e)),
                    }
                }
                
                debug!("Received {} bytes from Ingen", buffer.len());
                
                // Convert buffer to String
                let message = String::from_utf8_lossy(&buffer).to_string();
                trace!("Response buffer:\n{}", message);
                
                // Drop the socket guard before checking message content
                drop(socket_guard);
                
                // Check if this message contains actual content (patch:Put) - if not, ignore and receive again
                if !message.contains("a patch:Put") {
                    debug!("Message doesn't contain 'a patch:Put', ignoring and receiving next message");
                    continue;
                }
                
                return Ok(message);
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

        Ok(())
    }

    /// Disconnect two ports
    pub fn disconnect(&self, source: &str, destination: &str) -> Result<()> {
        info!("Disconnecting '{}' from '{}'", source, destination);

        // Build RDF message using protocol module
        let message = IngenProtocol::build_disconnect(source, destination)?;
        
        // Send to Ingen
        self.send_message(&message)?;

        Ok(())
    }

    /// Create an input port
    pub fn create_input_port(&self, port_name: &str, port_type: PortType) -> Result<String> {
        info!("Creating {:?} input port '{}'", port_type, port_name);

        // Build RDF message using protocol module
        let message = IngenProtocol::build_create_port(port_name, &port_type, &PortDirection::Input)?;
        
        // Send to Ingen
        self.send_message(&message)?;

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
        debug!("Setting raw engine state ({} bytes)", state_data.len());
        
        // Send data diretly to Ingen
        self.send_message(state_data)?;
        
        Ok(())
    }

    /// Get the current graph from Ingen
    pub fn get_graph(&self) -> Result<Graph> {
        debug!("Getting graph from Ingen");
        
        // Build RDF message to get the graph
        let message = IngenProtocol::build_get_state()?;
        
        // Send to Ingen
        self.send_message(&message)?;
        
        // Receive and parse response
        let response = self.receive_message()?;
        let graph = IngenProtocol::parse_graph(&response)?;
        
        trace!("Parsed graph: {} blocks, {} connections, {} system ports", 
               graph.blocks.len(), graph.connections.len(), graph.ports.len());
        trace!("Blocks: {:?}", graph.blocks);
        trace!("Connections: {:?}", graph.connections);
        trace!("System ports: {:?}", graph.ports);
        
        Ok(graph)
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
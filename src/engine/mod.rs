pub mod protocol;

use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::os::unix::net::UnixStream;
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

/// Engine module that encapsulates an Ingen instance
pub struct Engine {
    ingen_process: Option<std::process::Child>,
    socket: Option<UnixStream>,
}

impl Engine {
    /// Create a new engine instance
    pub fn new() -> Result<Self> {
        info!("Initializing Engine...");

        let mut engine = Self {
            ingen_process: None,
            socket: None,
        };

        // Start Ingen in the background
        //engine.start_ingen()?;
        
        // Connect to Ingen socket
        engine.connect_socket()?;

        // Create default ports
        let audio_in = engine.create_input_port("audio_in_1", PortType::Audio)?;    

        let audio_out = engine.create_output_port("audio_out_1", PortType::Audio)?;    

        engine.connect(&audio_in, &audio_out)?;

        // Discover available plugins
        //engine.discover_plugins()?;

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

    /// Connect to the Ingen Unix socket
    fn connect_socket(&mut self) -> Result<()> {
        info!("Connecting to Ingen socket...");
        
        let socket_path = "/tmp/ingen-traxdub.sock";
        
        // Retry connection a few times in case Ingen is still initializing
        let mut attempts = 0;
        let max_attempts = 10;
        
        loop {
            match UnixStream::connect(socket_path) {
                Ok(stream) => {
                    info!("Connected to Ingen socket");
                    self.socket = Some(stream);
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
    fn send_message(&mut self, message: &str) -> Result<()> {
        debug!("Sending message to Ingen: {}", message);
        
        if let Some(socket) = &mut self.socket {
            socket.write_all(message.as_bytes())
                .map_err(|e| anyhow!("Failed to write to Ingen socket: {}", e))?;
            socket.flush()
                .map_err(|e| anyhow!("Failed to flush Ingen socket: {}", e))?;
            Ok(())
        } else {
            Err(anyhow!("Not connected to Ingen socket"))
        }
    }

    /// Discover available LV2 plugins
    fn discover_plugins(&mut self) -> Result<()> {
        // TODO: Implement
        Ok(())
    }

    /// Create a new block (plugin instance)
    pub fn create_block(&mut self, plugin_uri: &str, block_id: &str) -> Result<()> {
        // TODO: Implement
        Ok(())
    }

    /// Duplicate a plugin instance
    pub fn duplicate_block(&mut self, source_block_id: &str, new_block_id: &str) -> Result<()> {
        // TODO: Implement
        Ok(())
    }

    /// Set a control parameter on a block
    pub fn set_control_parameter(
        &mut self,
        block_id: &str,
        parameter_name: &str,
        value: f32,
    ) -> Result<()> {
        // TODO: Implement
        Ok(())
    }

    /// Connect two ports
    pub fn connect(&mut self, source: &str, destination: &str) -> Result<()> {
        info!("Connecting '{}' to '{}'", source, destination);

        // Build RDF message using protocol module
        let message = IngenProtocol::build_connect(source, destination)?;
        
        // Send to Ingen
        self.send_message(&message)?;

        Ok(())
    }

    /// Disconnect two ports
    pub fn disconnect(&mut self, source: &str, destination: &str) -> Result<()> {
        info!("Disconnecting '{}' from '{}'", source, destination);

        // Build RDF message using protocol module
        let message = IngenProtocol::build_disconnect(source, destination)?;
        
        // Send to Ingen
        self.send_message(&message)?;

        Ok(())
    }

    /// Create an input port
    pub fn create_input_port(&mut self, port_name: &str, port_type: PortType) -> Result<String> {
        info!("Creating {:?} input port '{}'", port_type, port_name);

        // Build RDF message using protocol module
        let message = IngenProtocol::build_create_port(port_name, &port_type, &PortDirection::Input)?;
        
        // Send to Ingen
        self.send_message(&message)?;

        // Return the port path
        Ok(format!("ingen:/main/{}", port_name))
    }

    /// Create an output port
    pub fn create_output_port(&mut self, port_name: &str, port_type: PortType) -> Result<String> {
        info!("Creating {:?} output port '{}'", port_type, port_name);

        // Build RDF message using protocol module
        let message = IngenProtocol::build_create_port(port_name, &port_type, &PortDirection::Output)?;
        
        // Send to Ingen
        self.send_message(&message)?;

        // Return the port path
        Ok(format!("ingen:/main/{}", port_name))
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
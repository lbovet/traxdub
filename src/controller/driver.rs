use anyhow::{Result};
use jack::{Client, ClientOptions, ClosureProcessHandler, Control, MidiIn, ProcessScope, PortFlags};
use log::{debug, error, info, warn, trace};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}, Mutex};

/// Represents a JACK port with its ID and human-friendly name
#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
    pub short_name: String,
}

/// Represents a JACK client that provides input ports (source of audio/MIDI)
#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub ports: Vec<Port>,
}

/// Represents a JACK client that provides output ports (sink for audio/MIDI)
#[derive(Debug, Clone)]
pub struct Sink {
    pub name: String,
    pub ports: Vec<Port>,
}

/// Port type filter for JACK ports
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortType {
    Audio,
    Midi,
    All,
}

impl PortType {
    /// Convert to JACK type string filter
    fn to_jack_type_str(&self) -> Option<&'static str> {
        match self {
            PortType::Audio => Some("audio"),
            PortType::Midi => Some("midi"),
            PortType::All => None,
        }
    }
}

/// MIDI event types (excluding note events as per requirements)
#[derive(Debug, Clone)]
pub enum MidiEvent {
    ControlChange {
        channel: u8,
        control: u8,
        value: u8,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    PitchBend {
        channel: u8,
        value: u16,
    },
    AfterTouch {
        channel: u8,
        pressure: u8,
    },
    PolyAfterTouch {
        channel: u8,
        note: u8,
        pressure: u8,
    },
}

impl MidiEvent {
    /// Parse a raw MIDI message into a MidiEvent (excluding note events)
    pub fn from_raw(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        let status = data[0];
        let message_type = status & 0xF0;
        let channel = status & 0x0F;

        match message_type {
            // Control Change
            0xB0 => {
                if data.len() >= 3 {
                    Some(MidiEvent::ControlChange {
                        channel,
                        control: data[1],
                        value: data[2],
                    })
                } else {
                    None
                }
            }
            // Program Change
            0xC0 => {
                if data.len() >= 2 {
                    Some(MidiEvent::ProgramChange {
                        channel,
                        program: data[1],
                    })
                } else {
                    None
                }
            }
            // Channel Aftertouch
            0xD0 => {
                if data.len() >= 2 {
                    Some(MidiEvent::AfterTouch {
                        channel,
                        pressure: data[1],
                    })
                } else {
                    None
                }
            }
            // Pitch Bend
            0xE0 => {
                if data.len() >= 3 {
                    let value = ((data[2] as u16) << 7) | (data[1] as u16);
                    Some(MidiEvent::PitchBend { channel, value })
                } else {
                    None
                }
            }
            // Polyphonic Aftertouch
            0xA0 => {
                if data.len() >= 3 {
                    Some(MidiEvent::PolyAfterTouch {
                        channel,
                        note: data[1],
                        pressure: data[2],
                    })
                } else {
                    None
                }
            }
            // Note On/Off - explicitly ignored per requirements
            0x80 | 0x90 => {
                trace!("Ignoring note event");
                None
            }
            _ => {
                warn!("Unknown MIDI message type: 0x{:02X}", message_type);
                None
            }
        }
    }
}

/// MIDI receiver that connects to JACK and processes incoming MIDI events
pub struct Driver {
    _active_client_handle: Arc<AtomicBool>,
    client: Arc<Mutex<Option<Client>>>,
}

impl Driver {
    /// Sanitize port name for engine use
    /// Converts to lowercase and replaces sequences of special chars with single underscore
    pub fn sanitize_port_name(name: &str) -> String {
        let mut result = String::new();
        let mut last_was_special = false;
        
        for c in name.chars() {
            if c.is_alphanumeric() {
                result.push(c.to_lowercase().next().unwrap());
                last_was_special = false;
            } else {
                if !last_was_special && !result.is_empty() {
                    result.push('_');
                }
                last_was_special = true;
            }
        }
        
        // Remove trailing underscore if any
        if result.ends_with('_') {
            result.pop();
        }
        
        result
    }

    /// Create a new JACK driver instance
    pub fn new() -> Result<Self> {
        debug!("Initializing JACK driver...");

        // Create a JACK client for port queries
        let (query_client, _status) = Client::new("TraxDub Query", ClientOptions::NO_START_SERVER)
            .map_err(|e| anyhow::anyhow!("Failed to create JACK query client: {}", e))?;

        debug!("JACK query client created: {}", query_client.name());

        let client_storage = Arc::new(Mutex::new(Some(query_client)));
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        Ok(Self {
            _active_client_handle: shutdown_flag,
            client: client_storage,
        })
    }

    /// Start receiving MIDI events from JACK and return the receiver channel
    pub fn start(&self) -> Result<Receiver<MidiEvent>> {
        debug!("Starting JACK MIDI receiver...");

        let (event_sender, event_receiver) = channel();
        let shutdown_flag = Arc::clone(&self._active_client_handle);

        // Spawn a thread to keep the JACK client alive
        std::thread::spawn(move || {
            // Create JACK client
            let (client, _status) = Client::new("TraxDub Controller", ClientOptions::NO_START_SERVER)
                .expect("Failed to create JACK client");

            debug!("JACK client created: {}", client.name());

            // Create MIDI input port
            let midi_in = client
                .register_port("control", MidiIn::default())
                .expect("Failed to register MIDI input port");

            // Set up process callback
            let process_callback = move |_: &Client, ps: &ProcessScope| -> Control {
                // Get MIDI events from the port using iter() method
                for raw_event in midi_in.iter(ps) {
                    trace!("Raw MIDI bytes: {:?}", raw_event.bytes);
                    // Parse the MIDI event
                    if let Some(midi_event) = MidiEvent::from_raw(&raw_event.bytes) {
                        trace!("Parsed MIDI event: {:?}", midi_event);
                        // Send to channel
                        if let Err(e) = event_sender.send(midi_event) {
                            error!("Failed to send MIDI event: {}", e);
                        }
                    } else {
                        trace!("Ignored or unknown MIDI event");
                    }
                }

                Control::Continue
            };

            let process_handler = ClosureProcessHandler::new(process_callback);

            // Activate the client
            let active_client = client
                .activate_async((), process_handler)
                .expect("Failed to activate JACK client");

            debug!("JACK client activated");

            // Keep the thread alive to maintain the JACK connection
            while !shutdown_flag.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            
            debug!("JACK client closed");
            drop(active_client);
        });

        // Give the thread a moment to start
        std::thread::sleep(std::time::Duration::from_millis(100));

        Ok(event_receiver)
    }

    /// Get all JACK clients that provide input ports (sources)
    /// 
    /// # Arguments
    /// * `port_type` - Filter by port type (Audio, Midi, or All)
    pub fn get_sources(&self, port_type: PortType) -> Result<Vec<Source>> {
        let client_guard = self.client.lock().unwrap();
        let client = client_guard.as_ref()
            .ok_or_else(|| anyhow::anyhow!("JACK client not initialized"))?;

        let mut sources_map: std::collections::HashMap<String, Source> = std::collections::HashMap::new();

        // Get all ports with output flag (these are sources we can read from)
        let ports = client.ports(None, port_type.to_jack_type_str(), PortFlags::IS_OUTPUT);

        for port_name in ports {
            // Parse client name from port name (format: "client_name:port_name")
            if let Some((client_name, port_short_name)) = port_name.split_once(':') {
                // Skip TraxDub clients
                if client_name.starts_with("TraxDub") {
                    continue;
                }
                
                let entry = sources_map.entry(client_name.to_string())
                    .or_insert_with(|| Source {
                        name: client_name.to_string(),
                        ports: Vec::new(),
                    });

                entry.ports.push(Port {
                    name: port_name.to_string(),
                    short_name: port_short_name.to_string().split(":").last().unwrap_or("").to_string(),
                });
            }
        }

        let mut sources: Vec<Source> = sources_map.into_values().collect();
        
        // Sort sources alphabetically by name
        sources.sort_by(|a, b| a.name.cmp(&b.name));
        
        // Sort ports within each source alphabetically
        for source in &mut sources {
            source.ports.sort_by(|a, b| a.name.cmp(&b.name));
        }

        Ok(sources)
    }

    /// Get all JACK clients that provide output ports (sinks)
    /// 
    /// # Arguments
    /// * `port_type` - Filter by port type (Audio, Midi, or All)
    pub fn get_sinks(&self, port_type: PortType) -> Result<Vec<Sink>> {
        let client_guard = self.client.lock().unwrap();
        let client = client_guard.as_ref()
            .ok_or_else(|| anyhow::anyhow!("JACK client not initialized"))?;

        let mut sinks_map: std::collections::HashMap<String, Sink> = std::collections::HashMap::new();

        // Get all ports with input flag (these are sinks we can write to)
        let ports = client.ports(None, port_type.to_jack_type_str(), PortFlags::IS_INPUT);

        for port_name in ports {
            // Parse client name from port name (format: "client_name:port_name")
            if let Some((client_name, port_short_name)) = port_name.split_once(':') {
                // Skip TraxDub clients
                if client_name.starts_with("TraxDub") {
                    continue;
                }
                
                let entry = sinks_map.entry(client_name.to_string())
                    .or_insert_with(|| Sink {
                        name: client_name.to_string(),
                        ports: Vec::new(),
                    });

                entry.ports.push(Port {
                    name: port_name.to_string(),
                    short_name: port_short_name.to_string().split(":").last().unwrap_or("").to_string(),
                });
            }
        }

        let mut sinks: Vec<Sink> = sinks_map.into_values().collect();
        
        // Sort sinks alphabetically by name
        sinks.sort_by(|a, b| a.name.cmp(&b.name));
        
        // Sort ports within each sink alphabetically
        for sink in &mut sinks {
            sink.ports.sort_by(|a, b| a.name.cmp(&b.name));
        }

        Ok(sinks)
    }

    /// Connect two JACK ports
    /// 
    /// # Arguments
    /// * `source_port` - Reference to the source Port
    /// * `destination_port` - Reference to the destination Port
    pub fn connect_ports(&self, source_port: &Port, destination_port: &Port) -> Result<()> {
        let client_guard = self.client.lock().unwrap();
        let client = client_guard.as_ref()
            .ok_or_else(|| anyhow::anyhow!("JACK client not initialized"))?;

        debug!("Connecting JACK ports: {} -> {}", source_port.name, destination_port.name);
        
        match client.connect_ports_by_name(&source_port.name, &destination_port.name) {
            Ok(_) => {
                debug!("Successfully connected {} to {}", source_port.name, destination_port.name);
                Ok(())
            }
            Err(e) => {
                // Check if ports are already connected - this is not an error
                match e {
                    jack::Error::PortAlreadyConnected(_, _) => {
                        debug!("Ports already connected: {} -> {}", source_port.name, destination_port.name);
                        Ok(())
                    }
                    _ => Err(anyhow::anyhow!("Failed to connect ports: {:?}", e))
                }
            }
        }
    }

    /// Connect all MIDI input sources to the TraxDub Controller MIDI input port
    pub fn connect_all_midi_inputs(&self) -> Result<()> {
        debug!("Connecting all MIDI input sources to TraxDub Controller...");
        
        let sources = self.get_sources(PortType::Midi)?;
        let destination = Port {
            name: "TraxDub Controller:control".to_string(),
            short_name: "control".to_string(),
        };

        let mut connected_count = 0;
        for source in sources {
            for port in source.ports {
                match self.connect_ports(&port, &destination) {
                    Ok(_) => connected_count += 1,
                    Err(e) => warn!("Failed to connect {}: {}", port.name, e),
                }
            }
        }

        info!("Listening to {} MIDI input port(s)", connected_count);
        Ok(())
    }

    pub fn close(&self) {
        debug!("Signaling JACK client shutdown");
        self._active_client_handle.store(true, Ordering::SeqCst);
        // Give the thread time to cleanup
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_control_change() {
        let data = [0xB0, 0x07, 0x64]; // CC 7 (volume), value 100, channel 0
        let event = MidiEvent::from_raw(&data).unwrap();

        match event {
            MidiEvent::ControlChange {
                channel,
                control,
                value,
            } => {
                assert_eq!(channel, 0);
                assert_eq!(control, 7);
                assert_eq!(value, 100);
            }
            _ => panic!("Expected ControlChange event"),
        }
    }

    #[test]
    fn test_ignore_note_on() {
        let data = [0x90, 0x3C, 0x64]; // Note On, middle C, velocity 100
        let event = MidiEvent::from_raw(&data);
        assert!(event.is_none());
    }

    #[test]
    fn test_parse_program_change() {
        let data = [0xC0, 0x05]; // Program change to program 5, channel 0
        let event = MidiEvent::from_raw(&data).unwrap();

        match event {
            MidiEvent::ProgramChange { channel, program } => {
                assert_eq!(channel, 0);
                assert_eq!(program, 5);
            }
            _ => panic!("Expected ProgramChange event"),
        }
    }
}

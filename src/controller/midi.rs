use anyhow::{anyhow, Result};
use jack::{Client, ClientOptions, ClosureProcessHandler, Control, MidiIn, ProcessScope};
use log::{debug, error, info, warn};
use std::sync::mpsc::{channel, Receiver};

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
                debug!("Ignoring note event");
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
pub struct MidiReceiver;

impl MidiReceiver {
    /// Start receiving MIDI events from JACK and return the receiver channel
    pub fn start() -> Result<Receiver<MidiEvent>> {
        info!("Initializing JACK MIDI client...");

        let (event_sender, event_receiver) = channel();

        // Spawn a thread to keep the JACK client alive
        std::thread::spawn(move || {
            // Create JACK client
            let (client, _status) = Client::new("traxdub", ClientOptions::NO_START_SERVER)
                .expect("Failed to create JACK client");

            info!("JACK client created: {}", client.name());

            // Create MIDI input port
            let midi_in = client
                .register_port("midi_in", MidiIn::default())
                .expect("Failed to register MIDI input port");

            // Set up process callback
            let process_callback = move |_: &Client, ps: &ProcessScope| -> Control {
                // Get MIDI events from the port using iter() method
                for raw_event in midi_in.iter(ps) {
                    debug!("Raw MIDI bytes: {:?}", raw_event.bytes);
                    // Parse the MIDI event
                    if let Some(midi_event) = MidiEvent::from_raw(&raw_event.bytes) {
                        debug!("Parsed MIDI event: {:?}", midi_event);
                        // Send to channel
                        if let Err(e) = event_sender.send(midi_event) {
                            error!("Failed to send MIDI event: {}", e);
                        }
                    } else {
                        debug!("Ignored or unknown MIDI event");
                    }
                }

                Control::Continue
            };

            let process_handler = ClosureProcessHandler::new(process_callback);

            // Activate the client
            let active_client = client
                .activate_async((), process_handler)
                .expect("Failed to activate JACK client");

            info!("JACK client activated and receiving MIDI events");
            info!("Connect your MIDI controller to 'traxdub:midi_in' using qjackctl or jack_connect");

            // Keep the thread alive to maintain the JACK connection
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        // Give the thread a moment to start
        std::thread::sleep(std::time::Duration::from_millis(100));

        Ok(event_receiver)
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

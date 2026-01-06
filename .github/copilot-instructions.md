# TraxDub - GitHub Copilot Instructions

## Project Overview
TraxDub is a live music station application built in Rust. It provides a MIDI-controlled interface for managing audio processing graphs through an Ingen backend.

## Architecture

### Core Modules

1. **Controller Module** (`src/controller/`)
   - Main coordination hub that processes MIDI events
   - Implements state machine for application flow
   - Contains `midi` submodule for JACK MIDI integration
   - Manages base control configuration (~/.traxdub/base-control.json)
   - IMPORTANT: Controller does NOT process MIDI note events, only CC, program change, pitch bend, and aftertouch

2. **Engine Module** (`src/engine/`)
   - Encapsulates Ingen instance for audio graph management
   - Provides methods for: plugin listing, block creation/duplication, parameter control, connections
   - Handles graph persistence (load/save operations)
   - All operations should be designed to work with LV2 plugins through Ingen

3. **UI Module** (`src/ui/`)
   - Phase 1: Console-based output only
   - Receives signals from controller about state changes, plugin creation, connections
   - Will evolve to graphical interface in future phases

### State Machine Flow

The controller operates in these states:
- **Initializing**: Startup, checking for config
- **Learning modes**: LearningSelectionKnob → LearningSelectionButton → LearningBackButton
- **Ready**: Normal operation mode
- **Processing**: Active event handling

### Configuration
- Base controls stored in `~/.traxdub/base-control.json`
- Three essential controls: main selection knob (CC), main selection button, main back button
- Learning mode activated on first run when config doesn't exist

## Coding Guidelines

### Rust Best Practices
- Use `anyhow::Result` for error handling in application code
- Use `thiserror` for custom error types in library code
- Always use structured logging with the `log` crate
- Prefer `Arc` for shared state across threads
- Use channels (`std::sync::mpsc`) for inter-thread communication

### MIDI Event Handling
- Parse MIDI messages in the `midi` submodule
- Filter out note events (0x80, 0x90) - these are explicitly ignored
- Process: CC (0xB0), Program Change (0xC0), Pitch Bend (0xE0), Aftertouch (0xD0, 0xA0)
- Always validate MIDI message length before parsing

### Engine Integration
- Assume Ingen is running as a separate process
- Use appropriate paths for blocks: `/main/{block_id}`
- Plugin URIs follow LV2 format: `http://lv2plug.in/plugins/...`
- Connection format: `{block_path}:{port_name}`

### Testing
- Write unit tests for pure logic (MIDI parsing, state transitions)
- Use `mockall` for mocking external dependencies in tests
- Integration tests should use test fixtures for config files

## Development Workflow

### Adding New Features
1. Determine which module(s) are affected
2. Update state machine in controller if needed
3. Add engine methods if graph operations required
4. Update UI to display relevant information
5. Add tests for new functionality

### MIDI Control Mapping
- All MIDI learning should validate and store channel + CC/note number
- Control mappings should be serializable with serde
- Always provide user feedback through UI during learning

### Error Handling
- Controller errors: log and display to user via UI
- Engine errors: should be recoverable where possible
- MIDI errors: log warnings but don't crash the application

## Future Considerations
- UI will become graphical (possibly using egui or iced)
- More complex state machines for different operational modes
- Plugin preset management
- Session management and recall
- Real-time parameter automation

## Code Style
- Use `rustfmt` defaults
- Maximum line length: 100 characters
- Organize imports: std, external crates, internal modules
- Document public APIs with doc comments (`///`)
- Keep functions focused and under 50 lines when possible

## Dependencies
- **jack**: JACK audio connection kit bindings
- **serde/serde_json**: Configuration serialization
- **anyhow/thiserror**: Error handling
- **log/env_logger**: Logging infrastructure

## Testing and Running
```bash
# Build the project
cargo build

# Run with logging
RUST_LOG=info cargo run

# Run tests
cargo test

# Run with specific log level
RUST_LOG=debug cargo run
```

## Common Patterns

### Adding a New Engine Operation
```rust
pub fn new_operation(&mut self, param: &str) -> Result<()> {
    info!("Performing new operation with {}", param);
    // Validate inputs
    // Send command to Ingen
    // Update internal state
    Ok(())
}
```

### Processing Controller Events
```rust
fn process_event(&mut self, event: midi::MidiEvent) -> Result<()> {
    match self.state {
        ControllerState::Ready => {
            // Apply rules based on event content
            // Call engine methods as needed
            // Update UI
        }
        _ => { /* handle other states */ }
    }
    Ok(())
}
```

### Updating UI
```rust
self.ui.signal_plugin_created(&plugin_name, &block_id)?;
self.ui.signal_state_change("NewState")?;
```

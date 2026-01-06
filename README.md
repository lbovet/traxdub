# TraxDub

A live music station application with MIDI control and real-time audio processing.

## Overview

TraxDub is a Rust-based application that provides a MIDI-controlled interface for managing audio processing graphs. It uses JACK for MIDI input and Ingen as the audio graph engine backend.

## Features

- **MIDI Control**: Receives and processes MIDI events (CC, program change, pitch bend, aftertouch) via JACK
- **Audio Graph Management**: Create, connect, and control LV2 plugin instances through Ingen
- **Learning Mode**: Automatically configures base controls on first startup
- **State Machine**: Robust state management for different operational modes
- **Modular Architecture**: Clean separation between controller, engine, and UI layers

## Architecture

### Modules

- **Controller**: Core coordination module that processes MIDI events and manages application state
  - **MIDI Submodule**: Handles JACK MIDI integration and event parsing
- **Engine**: Encapsulates Ingen instance for audio graph manipulation
  - List available plugins
  - Create and duplicate blocks (plugin instances)
  - Set control parameters
  - Manage connections
  - Load/save graph state
- **UI**: User interface (Phase 1: console output)
  - Display state changes
  - Show plugin and connection information
  - Provide user prompts during learning mode

### State Flow

```
Initializing → Learning Mode (if no config) → Ready → Processing
                ↓
         LearningSelectionKnob
                ↓
         LearningSelectionButton
                ↓
         LearningBackButton
                ↓
         Save Config → Ready
```

## Prerequisites

- Rust 1.70 or later
- JACK Audio Connection Kit
- Ingen (LV2 host)
- LV2 plugins

### Linux Installation

```bash
# Ubuntu/Debian
sudo apt-get install jackd2 libjack-jackd2-dev ingen lv2-dev

# Arch Linux
sudo pacman -S jack2 ingen lv2

# Fedora
sudo dnf install jack-audio-connection-kit-devel ingen lv2-devel
```

## Building

```bash
# Clone the repository
git clone <repository-url>
cd traxdub

# Build the project
cargo build --release

# Run
cargo run --release
```

## Usage

### First Run (Learning Mode)

On first startup, TraxDub will guide you through configuring the base controls:

1. Turn the **main selection knob** (any MIDI CC controller)
2. Press the **main selection button**
3. Press the **main back button**

These assignments are saved to `~/.traxdub/base-control.json`

### Normal Operation

After configuration, TraxDub will:
- Connect to JACK for MIDI input
- Initialize the Ingen engine
- Listen for MIDI events on the `traxdub:midi_in` port

### Connecting MIDI Controller

Use JACK tools to connect your MIDI controller:

```bash
# List MIDI ports
jack_lsp -t

# Connect your controller to TraxDub
jack_connect <your-controller>:midi_out traxdub:midi_in

# Or use QjackCtl GUI
qjackctl
```

## Configuration

### Base Control Configuration

Located at `~/.traxdub/base-control.json`:

```json
{
  "main_selection_knob": {
    "channel": 0,
    "cc_or_note": 1,
    "control_type": "CC"
  },
  "main_selection_button": {
    "channel": 0,
    "cc_or_note": 16,
    "control_type": "Button"
  },
  "main_back_button": {
    "channel": 0,
    "cc_or_note": 17,
    "control_type": "Button"
  }
}
```

To reconfigure, delete this file and restart TraxDub.

## Development

### Running with Debug Logging

```bash
RUST_LOG=debug cargo run
```

### Running Tests

```bash
cargo test
```

### Project Structure

```
traxdub/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point
│   ├── controller/
│   │   ├── mod.rs           # Controller implementation
│   │   └── midi.rs          # MIDI event handling
│   ├── engine/
│   │   └── mod.rs           # Ingen engine wrapper
│   └── ui/
│       └── mod.rs           # UI implementation (console)
└── .github/
    └── copilot-instructions.md
```

## Roadmap

### Phase 1 (Current)
- [x] Basic MIDI input via JACK
- [x] Learning mode for base controls
- [x] Console-based UI
- [x] Engine module structure

### Phase 2 (Planned)
- [ ] Full Ingen integration
- [ ] Plugin instance management
- [ ] Connection management
- [ ] Graph persistence

### Phase 3 (Future)
- [ ] Graphical UI
- [ ] Advanced state machine
- [ ] Session management
- [ ] Preset system
- [ ] Parameter automation

## Contributing

Contributions are welcome! Please read the Copilot instructions in `.github/copilot-instructions.md` for development guidelines.

## License

[Specify your license here]

## Acknowledgments

- [Ingen](https://drobilla.net/software/ingen.html) - LV2 plugin host
- [JACK Audio Connection Kit](https://jackaudio.org/)
- [LV2](https://lv2plug.in/) - Audio plugin standard

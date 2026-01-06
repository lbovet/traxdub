# TraxDub

A live music station application with MIDI control and real-time audio processing.

## Overview

TraxDub is a Rust-based application that provides a MIDI-controlled interface for managing audio processing graphs. It uses JACK for MIDI input and Ingen as the audio graph engine backend.

## Features

- **MIDI Control**: Receives and processes MIDI events via JACK
- **Audio Graph Management**: Create, connect, and control LV2 plugin instances
- **No Configuration**: Learns MIDI bindings during usage 

## Prerequisites

- JACK Audio Connection Kit
- LV2 plugins

## Development

### Building

```bash
# Clone the repository
git clone <repository-url>
cd traxdub

# Build the project
cargo build --release

# Run
cargo run --release
```

### Running with Debug Logging

```bash
RUST_LOG=debug cargo run
```

### Running Tests

```bash
cargo test
```

## Contributing

Contributions are welcome! Please read the Copilot instructions in `.github/copilot-instructions.md` for development guidelines.

## License

GNU General Public License v3

## Acknowledgments

- [Ingen](https://drobilla.net/software/ingen.html) - LV2 plugin host
- [JACK Audio Connection Kit](https://jackaudio.org/)
- [LV2](https://lv2plug.in/) - Audio plugin standard

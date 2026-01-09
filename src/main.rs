mod controller;
mod engine;
mod ui;

use anyhow::Result;
use clap::Parser;
use log::info;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

/// TraxDub - Live music station application
#[derive(Parser, Debug)]
#[command(name = "traxdub")]
#[command(about = "A live music station application with MIDI control", long_about = None)]
struct Args {
    /// Force re-initialization of base controls
    #[arg(short, long)]
    init: bool,
}

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Parse command-line arguments
    let args = Args::parse();
    
    info!("Starting TraxDub...");
    
    // Set up Ctrl-C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        info!("Received Ctrl-C, shutting down...");
        r.store(false, Ordering::SeqCst);
    })?;
    
    // Initialize modules
    let ui = Arc::new(ui::UI::new());
    let engine = Arc::new(engine::Engine::new()?);
    let mut controller = controller::Controller::new(ui.clone(), engine.clone(), args.init)?;
    
    info!("TraxDub initialized successfully");
    
    // Run the controller with graceful shutdown
    let result = controller.run_until_signal(running);
    
    // Explicitly drop engine to ensure clean shutdown
    info!("Dropping engine...");
    drop(engine);
    
    result
}

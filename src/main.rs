mod controller;
mod engine;
mod ui;
use anyhow::Result;
use clap::Parser;
use log::{debug, info};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

/// TraxDub - Live music station application
#[derive(Parser, Debug)]
#[command(name = "traxdub")]
#[command(about = "A live music station application with MIDI control", long_about = None)]
struct Args {
    /// Force re-initialization of base controls
    #[arg(short, long)]
    init: bool,
    
    /// Use external Ingen instance (don't start built-in Ingen process)
    #[arg(short, long)]
    external: bool,
    
    /// Start with a new session (don't load last saved state)
    #[arg(short, long)]
    new: bool,
}

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Parse command-line arguments
    let args = Args::parse();
    
    debug!("Starting TraxDub...");
    
    // Set up Ctrl-C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    // Initialize modules
    let ui = Arc::new(ui::UI::new());
    let engine = Arc::new(engine::Engine::new(args.external)?);
    let mut controller = controller::Controller::new(ui.clone(), engine.clone(), args.init, args.new)?;
    
    ctrlc::set_handler(move || {
        info!("Received Ctrl-C, shutting down");
        r.store(false, Ordering::SeqCst);
    })?;

    debug!("TraxDub initialized");
    
    // Use scoped threads to avoid Send requirement
    let result = std::thread::scope(|s| {
        // Start the controller in a background thread
        let controller_running = running.clone();
        s.spawn(move || {
            let _ = controller.run_until_signal(controller_running);

            // Close the UI
            debug!("Closing UI...");
            let _ = ui::window::close();
        });
        
        // Run the UI window on the main thread (required for most platforms)
        let ui_result = ui::window::run(running.clone());

        // Return the first error if any occurred
        ui_result
    });
    
    result
}

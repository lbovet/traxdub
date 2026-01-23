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
    ctrlc::set_handler(move || {
        info!("Received Ctrl-C, shutting down");
        r.store(false, Ordering::SeqCst);
    })?;
    
    // Initialize modules
    let ui = Arc::new(ui::UI::new());
    let engine = Arc::new(engine::Engine::new(args.external)?);
    let mut controller = controller::Controller::new(ui.clone(), engine.clone(), args.init, args.new)?;
    
    debug!("TraxDub initialized");
    
    // Use scoped threads to avoid Send requirement
    let result = std::thread::scope(|s| {
        // Start the controller in a background thread
        let controller_running = running.clone();
        let controller_handle = s.spawn(move || {
            controller.run_until_signal(controller_running)
        });
        
        // Run the UI window on the main thread (required for most platforms)
        let ui_result = ui::window::run();
        
        // Wait for controller to finish
        let controller_result = controller_handle.join()
            .unwrap_or_else(|e| Err(anyhow::anyhow!("Controller thread panicked: {:?}", e)));
        
        // Return the first error if any occurred
        ui_result?;
        controller_result
    });
    
    // Explicitly drop engine to ensure clean shutdown
    debug!("Dropping engine...");
    drop(engine);
    
    result
}

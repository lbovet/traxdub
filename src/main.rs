mod controller;
mod engine;
mod ui;

use anyhow::Result;
use clap::Parser;
use log::info;
use std::sync::Arc;

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
    
    // Initialize modules
    let ui = Arc::new(ui::UI::new());
    let engine = Arc::new(engine::Engine::new()?);
    let mut controller = controller::Controller::new(ui.clone(), engine.clone(), args.init)?;
    
    info!("TraxDub initialized successfully");
    
    // Run the controller
    controller.run()?;
    
    Ok(())
}

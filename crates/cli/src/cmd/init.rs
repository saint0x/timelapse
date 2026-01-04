//! Initialize Timelapse in a repository

use anyhow::Result;
use core::store::Store;
use std::env;

pub async fn run() -> Result<()> {
    // Get current directory
    let current_dir = env::current_dir()?;

    println!("Initializing Timelapse repository at {}", current_dir.display());

    // Initialize store
    match Store::init(&current_dir) {
        Ok(_) => {
            println!("Successfully initialized Timelapse repository");
            println!();
            println!("Created .tl/ directory structure:");
            println!("  - .tl/objects/blobs/    (file content storage)");
            println!("  - .tl/objects/trees/    (directory tree storage)");
            println!("  - .tl/journal/          (checkpoint history)");
            println!("  - .tl/refs/pins/        (pinned checkpoints)");
            println!("  - .tl/state/            (state files)");
            println!();
            println!("Next steps:");
            println!("  - Run 'tl status' to check repository status");
            println!("  - Run 'tl info' to see repository statistics");
            Ok(())
        }
        Err(e) => {
            if e.to_string().contains("already initialized") {
                println!("Error: Timelapse repository already initialized");
                println!("Location: {}/.tl/", current_dir.display());
                std::process::exit(1);
            } else {
                Err(e)
            }
        }
    }
}

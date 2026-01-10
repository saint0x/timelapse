//! Configuration management command
//!
//! Provides CLI interface to view and edit system configuration.

use crate::system_config::{self, SystemConfig};
use anyhow::{Context, Result};
use owo_colors::OwoColorize;

/// List all configuration values
pub async fn run_list() -> Result<()> {
    let config = system_config::load()?;
    let config_path = system_config::config_file_path()
        .context("Could not determine config file path")?;

    println!("{}", "System Configuration".bold());
    println!("{}: {}\n", "Location".dimmed(), config_path.display().dimmed());

    println!("{}", "[daemon]".yellow());
    println!(
        "  {} = {} {}",
        "checkpoint_interval_secs".cyan(),
        config.daemon.checkpoint_interval_secs,
        format!("({}s)", config.daemon.checkpoint_interval_secs).dimmed()
    );
    println!(
        "  {} = {}",
        "auto_gc_enabled".cyan(),
        config.daemon.auto_gc_enabled
    );
    println!(
        "  {} = {} {}",
        "auto_gc_interval_secs".cyan(),
        config.daemon.auto_gc_interval_secs,
        format!("({}s = {} min)",
            config.daemon.auto_gc_interval_secs,
            config.daemon.auto_gc_interval_secs / 60
        ).dimmed()
    );
    println!(
        "  {} = {}",
        "auto_gc_checkpoint_threshold".cyan(),
        config.daemon.auto_gc_checkpoint_threshold
    );

    println!("\n{}", "[gc]".yellow());
    println!(
        "  {} = {}",
        "retain_count".cyan(),
        config.gc.retain_count
    );
    println!(
        "  {} = {} {}",
        "retain_hours".cyan(),
        config.gc.retain_hours,
        if config.gc.retain_hours == 0 {
            "(no time limit)".dimmed().to_string()
        } else {
            format!("({}h)", config.gc.retain_hours).dimmed().to_string()
        }
    );
    println!(
        "  {} = {}",
        "retain_pins".cyan(),
        config.gc.retain_pins
    );

    println!("\n{}", "Valid Ranges:".bold());
    println!("  checkpoint_interval_secs: 1-3600");
    println!("  auto_gc_interval_secs: 60-86400");
    println!("  auto_gc_checkpoint_threshold: 100-100,000");
    println!("  retain_count: 10-1,000,000");
    println!("  retain_hours: 0-8760 (0 = no time limit)");

    Ok(())
}

/// Get a single configuration value
pub async fn run_get(key: &str) -> Result<()> {
    let config = system_config::load()?;

    let value = match key {
        "daemon.checkpoint_interval_secs" => config.daemon.checkpoint_interval_secs.to_string(),
        "daemon.auto_gc_enabled" => config.daemon.auto_gc_enabled.to_string(),
        "daemon.auto_gc_interval_secs" => config.daemon.auto_gc_interval_secs.to_string(),
        "daemon.auto_gc_checkpoint_threshold" => config.daemon.auto_gc_checkpoint_threshold.to_string(),
        "gc.retain_count" => config.gc.retain_count.to_string(),
        "gc.retain_hours" => config.gc.retain_hours.to_string(),
        "gc.retain_pins" => config.gc.retain_pins.to_string(),
        _ => anyhow::bail!(
            "Unknown config key: {}. Use 'tl config --list' to see available keys.",
            key
        ),
    };

    println!("{}", value);
    Ok(())
}

/// Set a configuration value
pub async fn run_set(key: &str, value: &str) -> Result<()> {
    let mut config = system_config::load()?;

    match key {
        "daemon.checkpoint_interval_secs" => {
            let val: u64 = value.parse()
                .context("Invalid value: must be a positive integer")?;
            config.daemon.checkpoint_interval_secs = val;
        }
        "daemon.auto_gc_enabled" => {
            let val: bool = value.parse()
                .context("Invalid value: must be 'true' or 'false'")?;
            config.daemon.auto_gc_enabled = val;
        }
        "daemon.auto_gc_interval_secs" => {
            let val: u64 = value.parse()
                .context("Invalid value: must be a positive integer")?;
            config.daemon.auto_gc_interval_secs = val;
        }
        "daemon.auto_gc_checkpoint_threshold" => {
            let val: usize = value.parse()
                .context("Invalid value: must be a positive integer")?;
            config.daemon.auto_gc_checkpoint_threshold = val;
        }
        "gc.retain_count" => {
            let val: usize = value.parse()
                .context("Invalid value: must be a positive integer")?;
            config.gc.retain_count = val;
        }
        "gc.retain_hours" => {
            let val: u64 = value.parse()
                .context("Invalid value: must be a non-negative integer")?;
            config.gc.retain_hours = val;
        }
        "gc.retain_pins" => {
            let val: bool = value.parse()
                .context("Invalid value: must be 'true' or 'false'")?;
            config.gc.retain_pins = val;
        }
        _ => anyhow::bail!(
            "Unknown config key: {}. Use 'tl config --list' to see available keys.",
            key
        ),
    }

    // Validate before saving
    config.validate()
        .context("Invalid configuration value")?;

    system_config::save(&config)?;

    println!("{} {} = {}", "✓".green(), key.cyan(), value);
    println!(
        "{}",
        "Note: Restart daemon for changes to take effect (tl stop && tl start)".yellow()
    );

    Ok(())
}

/// Show the config file path and optionally create it
pub async fn run_path(create: bool) -> Result<()> {
    let config_path = system_config::config_file_path()
        .context("Could not determine config file path")?;

    if create && !config_path.exists() {
        system_config::init_if_missing()?;
        println!("{} Created config file at: {}", "✓".green(), config_path.display());
    } else if config_path.exists() {
        println!("{}", config_path.display());
    } else {
        println!("{}", config_path.display());
        println!("{}", "File does not exist. Use --create to create it.".yellow());
    }

    Ok(())
}

/// Show example configuration
pub async fn run_example() -> Result<()> {
    let example = system_config::example_config();
    println!("{}", example);
    Ok(())
}

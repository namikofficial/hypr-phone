//! `hypr-phone config` subcommands: path, init, status, migrate.

use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::cli::ConfigAction;
use crate::config::{Config, CURRENT_CONFIG_VERSION, LEGACY_CONFIG_VERSION};

pub fn run(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Path => {
            let path = default_path()?;
            println!("{}", path.display());
            Ok(())
        }
        ConfigAction::Init => init(),
        ConfigAction::Status => status(),
        ConfigAction::Migrate => migrate(),
    }
}

fn default_path() -> Result<PathBuf> {
    let dir = dirs::config_dir().ok_or_else(|| anyhow!("could not resolve config directory"))?;
    Ok(dir.join("hypr-phone").join("config.toml"))
}

fn init() -> Result<()> {
    let path = default_path()?;
    if path.exists() {
        println!("Config already exists at {}", path.display());
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let template = Config::default().to_toml_string()?;
    fs::write(&path, template).with_context(|| format!("failed to write {}", path.display()))?;
    println!("Initialized config at {}", path.display());
    Ok(())
}

fn status() -> Result<()> {
    let cfg = Config::load_default()?;
    println!("config_version = {}", cfg.config_version);
    println!("current_version = {CURRENT_CONFIG_VERSION}");
    println!(
        "migration_needed = {}",
        if cfg.is_current_version() {
            "no"
        } else {
            "yes"
        }
    );
    println!("legacy_version = {LEGACY_CONFIG_VERSION}");
    println!(
        "default_alias = {}",
        cfg.mirror.default_alias.as_deref().unwrap_or("(none)")
    );
    println!("default_profile = {}", cfg.mirror.profile);
    println!("aliases = {}", cfg.devices.entries.len());
    println!("known_endpoints = {}", cfg.reconnect.known_endpoints.len());
    Ok(())
}

fn migrate() -> Result<()> {
    let cfg = Config::load_default()?;
    let path = cfg.save_default()?;
    println!(
        "Migrated config to v{CURRENT_CONFIG_VERSION} at {}",
        path.display()
    );
    Ok(())
}

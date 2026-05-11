mod adb;
mod cli;
mod config;
mod errors;
mod hyprland;
mod menu;
mod module_output;
mod scrcpy;

use std::{fs, path::PathBuf};

use anyhow::{anyhow, bail, Context};
use clap::Parser;

use crate::errors::{missing_dependency, Result};

fn main() -> Result<()> {
    let args = cli::Cli::parse();

    match args.command.unwrap_or(cli::Command::Run) {
        cli::Command::Run => {
            let _ = config::Config::load_default()?;
            let output = module_output::ModuleOutput::placeholder("run");
            println!("{}", menu::build_menu(&output));
        }
        cli::Command::Doctor => {
            run_doctor()?;
        }
        cli::Command::Devices { json } => {
            run_devices(json)?;
        }
        cli::Command::Pair {
            endpoint,
            pairing_code,
        } => {
            let output = adb::pair(&endpoint, &pairing_code)?;
            print_command_output(&output);
        }
        cli::Command::Connect { endpoint } => {
            let output = adb::connect(&endpoint)?;
            print_command_output(&output);
        }
        cli::Command::Disconnect { serial } => {
            let output = adb::disconnect(&serial)?;
            print_command_output(&output);
        }
        cli::Command::Config { command } => match command {
            cli::ConfigCommand::Path => {
                println!("{}", default_config_path()?.display());
            }
            cli::ConfigCommand::Init => {
                init_config_file()?;
            }
        },
    }

    Ok(())
}

fn run_doctor() -> Result<()> {
    let mut missing_required = Vec::new();

    match adb::adb_path() {
        Ok(path) => println!("[ok] adb: {}", path.display()),
        Err(err) => {
            eprintln!("[missing] adb: {err}");
            missing_required.push("adb");
        }
    }

    check_dependency(
        "scrcpy",
        "Install `scrcpy` to mirror your Android device.",
        &mut missing_required,
    );
    check_dependency(
        "hyprctl",
        "Install Hyprland or ensure `hyprctl` is available in PATH.",
        &mut missing_required,
    );
    check_dependency(
        "wl-copy",
        "Install `wl-clipboard` package for clipboard support.",
        &mut missing_required,
    );
    check_dependency(
        "wl-paste",
        "Install `wl-clipboard` package for clipboard support.",
        &mut missing_required,
    );
    check_dependency(
        "notify-send",
        "Install `libnotify` to enable desktop notifications.",
        &mut missing_required,
    );

    let rofi = which::which("rofi").ok();
    let wofi = which::which("wofi").ok();
    match (rofi, wofi) {
        (Some(path), _) => println!("[ok] rofi: {}", path.display()),
        (None, Some(path)) => println!("[ok] wofi: {}", path.display()),
        (None, None) => {
            let err = missing_dependency(
                "rofi/wofi",
                "Install either `rofi` or `wofi` for menu selection.",
            );
            eprintln!("[missing] rofi/wofi: {err}");
            missing_required.push("rofi or wofi");
        }
    }

    match which::which("kdeconnect-cli") {
        Ok(path) => println!("[ok] kdeconnect-cli (optional): {}", path.display()),
        Err(_) => eprintln!(
            "[warn] kdeconnect-cli (optional): not found in PATH. KDE Connect features will be unavailable."
        ),
    }

    if missing_required.is_empty() {
        println!("Doctor passed. All required dependencies are available.");
        Ok(())
    } else {
        bail!(
            "Doctor failed. Missing required dependencies: {}.",
            missing_required.join(", ")
        )
    }
}

fn check_dependency(binary: &'static str, hint: &'static str, missing: &mut Vec<&'static str>) {
    match which::which(binary) {
        Ok(path) => println!("[ok] {binary}: {}", path.display()),
        Err(_) => {
            eprintln!("[missing] {binary}: {}", missing_dependency(binary, hint));
            missing.push(binary);
        }
    }
}

fn run_devices(as_json: bool) -> Result<()> {
    let devices = adb::devices()?;

    if as_json {
        println!("{}", serde_json::to_string_pretty(&devices)?);
        return Ok(());
    }

    if devices.is_empty() {
        println!("No adb devices detected.");
        return Ok(());
    }

    for device in devices {
        let mut line = format!("{} ({})", device.serial, device.state);
        if let Some(model) = device.model {
            line.push_str(&format!(" model={model}"));
        }
        if let Some(product) = device.product {
            line.push_str(&format!(" product={product}"));
        }
        println!("{line}");
    }

    Ok(())
}

fn print_command_output(output: &str) {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        println!("Command completed successfully.");
    } else {
        println!("{trimmed}");
    }
}

fn default_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Unable to determine the system config directory."))?;
    Ok(config_dir.join("hypr-phone").join("config.toml"))
}

fn init_config_file() -> Result<()> {
    let path = default_config_path()?;
    if path.exists() {
        println!("Config already exists at {}", path.display());
        return Ok(());
    }

    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("Failed to determine parent directory for config path."))?;

    fs::create_dir_all(parent).with_context(|| {
        format!(
            "Failed to create config directory at {}.",
            parent.display()
        )
    })?;

    let config_template = config::Config::default().to_toml_string()?;
    fs::write(&path, config_template)
        .with_context(|| format!("Failed to write config file at {}.", path.display()))?;

    println!("Initialized config at {}", path.display());
    Ok(())
}

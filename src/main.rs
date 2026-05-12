mod adb;
mod cli;
mod config;
mod errors;
mod hyprland;
mod menu;
mod module_output;
mod scrcpy;

use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    process::Command,
};

use anyhow::{anyhow, bail, Context};
use clap::Parser;

use crate::errors::{missing_dependency, Result};

fn main() -> Result<()> {
    let args = cli::Cli::parse();

    match args.command.unwrap_or(cli::Command::Run) {
        cli::Command::Run => {
            run_menu()?;
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
            run_connect(endpoint)?;
        }
        cli::Command::Disconnect { serial } => {
            let output = adb::disconnect(&serial)?;
            print_command_output(&output);
        }
        cli::Command::Mirror { device, profile } => {
            run_mirror(device, profile)?;
        }
        cli::Command::Module => {
            println!("{}", module_output::fast_status_json());
        }
        cli::Command::Menu => {
            run_menu()?;
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

fn run_mirror(device: Option<String>, profile: Option<String>) -> Result<()> {
    let config = config::Config::load_default()?;
    let launch = scrcpy::launch_mirror(
        &config,
        scrcpy::MirrorRequest {
            device_serial: device.as_deref(),
            profile: profile.as_deref(),
        },
    )?;

    let target = launch
        .resolved
        .device_serial
        .as_deref()
        .map(|serial| format!(" for `{serial}`"))
        .unwrap_or_default();
    println!(
        "Started scrcpy (pid {}) with profile `{}`{}.",
        launch.child.id(),
        launch.resolved.profile_name,
        target
    );

    if let Some(error) = launch.hyprland_error {
        eprintln!("[warn] Hyprland placement could not be applied: {error}");
    }

    Ok(())
}

fn run_menu() -> Result<()> {
    if let Some(output) = menu::launch_and_execute_v01_menu()? {
        print_command_output(&output);
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

fn run_connect(endpoint: Option<String>) -> Result<()> {
    if let Some(endpoint) = endpoint {
        let output = adb::connect(&endpoint).with_context(|| {
            format!(
                "Failed to connect to `{endpoint}`. Verify the endpoint and ensure Wireless debugging is enabled."
            )
        })?;
        print_command_output(&output);
        return Ok(());
    }

    run_guided_wireless_connect()
}

fn run_guided_wireless_connect() -> Result<()> {
    println!("Guided wireless ADB setup");

    let pair_endpoint_input =
        prompt_required_input("Pair endpoint (ip[:port], default port 37123)")?;
    let pair_endpoint = adb::normalize_endpoint(&pair_endpoint_input, 37123);
    let pairing_code = prompt_required_input("Pairing code")?;

    print_pairing_qr_helper(&pair_endpoint, &pairing_code);
    println!("Running: adb pair {pair_endpoint}");
    let pair_output = adb::pair(&pair_endpoint, &pairing_code).with_context(|| {
        format!(
            "Pairing failed for `{pair_endpoint}`. Confirm the pair endpoint and code shown in Android Wireless debugging."
        )
    })?;
    print_command_output(&pair_output);

    let default_connect_endpoint = format!("{}:5555", endpoint_ip(&pair_endpoint));
    let connect_prompt = format!("Connect endpoint (ip[:port]) [{default_connect_endpoint}]");
    let connect_endpoint_input = prompt_with_default(&connect_prompt, &default_connect_endpoint)?;
    let connect_endpoint = adb::normalize_endpoint(&connect_endpoint_input, 5555);

    println!("Running: adb connect {connect_endpoint}");
    let connect_output = adb::connect(&connect_endpoint).with_context(|| {
        format!(
            "Connect failed for `{connect_endpoint}`. Ensure the device stayed on the same network and Wireless debugging is still enabled."
        )
    })?;
    print_command_output(&connect_output);
    Ok(())
}

fn prompt_required_input(prompt: &str) -> Result<String> {
    loop {
        let Some(value) = prompt_line(prompt)? else {
            bail!(
                "Interactive input was closed. Re-run with `hypr-phone connect <ip:port>` for non-interactive use."
            );
        };

        if value.trim().is_empty() {
            eprintln!("Input cannot be empty.");
            continue;
        }

        return Ok(value);
    }
}

fn prompt_with_default(prompt: &str, default: &str) -> Result<String> {
    let Some(value) = prompt_line(prompt)? else {
        bail!(
            "Interactive input was closed. Re-run with `hypr-phone connect <ip:port>` for non-interactive use."
        );
    };

    if value.trim().is_empty() {
        Ok(default.to_string())
    } else {
        Ok(value)
    }
}

fn prompt_line(prompt: &str) -> Result<Option<String>> {
    print!("{prompt}: ");
    io::stdout()
        .flush()
        .context("failed to flush prompt to terminal")?;

    let mut input = String::new();
    let bytes_read = io::stdin()
        .read_line(&mut input)
        .context("failed to read interactive input")?;
    if bytes_read == 0 {
        return Ok(None);
    }

    Ok(Some(input.trim().to_string()))
}

fn endpoint_ip(endpoint: &str) -> &str {
    endpoint
        .split_once(':')
        .map(|(ip, _)| ip)
        .unwrap_or(endpoint)
}

fn print_pairing_qr_helper(pair_endpoint: &str, pairing_code: &str) {
    let payload = format!("WIFI:T:ADB;S:{pair_endpoint};P:{pairing_code};;");
    println!("Pairing QR payload: {payload}");

    if which::which("qrencode").is_err() {
        println!("(Tip: install `qrencode` to render this payload as a terminal QR helper.)");
        return;
    }

    match Command::new("qrencode")
        .args(["-t", "ANSIUTF8"])
        .arg(&payload)
        .output()
    {
        Ok(output) if output.status.success() => {
            let rendered = String::from_utf8_lossy(&output.stdout);
            if rendered.trim().is_empty() {
                eprintln!("[warn] qrencode produced empty output. Use the payload shown above.");
            } else {
                println!("{rendered}");
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                eprintln!("[warn] qrencode failed. Use the payload shown above.");
            } else {
                eprintln!("[warn] qrencode failed: {stderr}. Use the payload shown above.");
            }
        }
        Err(err) => {
            eprintln!("[warn] Failed to run qrencode: {err}. Use the payload shown above.");
        }
    }
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

    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create config directory at {}.", parent.display()))?;

    let config_template = config::Config::default().to_toml_string()?;
    fs::write(&path, config_template)
        .with_context(|| format!("Failed to write config file at {}.", path.display()))?;

    println!("Initialized config at {}", path.display());
    Ok(())
}

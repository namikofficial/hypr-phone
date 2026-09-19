//! `hypr-phone` — Hyprland-native Android presence layer.

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use hypr_phone::{
    cli::{
        Cli, Command, CompatClipboardAction, CompatCommand, CompatKdeAction, CompatMediaAction,
        ConfigAction, ControlAction, RecordAction,
    },
    commands, services,
};
use std::{io, process::Command as StdCommand};

fn main() -> Result<()> {
    let args = Cli::parse();
    run_command(args.command)
}

fn run_command(cmd: Option<Command>) -> Result<()> {
    // Default: contextual menu.
    let cmd = cmd.unwrap_or(Command::Menu);

    match cmd {
        Command::Menu => commands::menu::run(),
        Command::Toggle {
            profile,
            app,
            no_mirror,
        } => commands::toggle::run(commands::toggle::ToggleOptions {
            profile,
            app,
            no_mirror,
        }),
        Command::Status { json, waybar } => commands::status::run(json, waybar),
        Command::App { package } => commands::app::run(package),
        Command::Send { thing } => commands::send::run(thing),
        Command::Record { action } => commands::record::run(action),
        Command::Screenshot { output } => commands::screenshot::run(output),
        Command::Control { action } => commands::control::run(action),
        Command::Clipboard { action } => commands::clipboard::run(action),
        Command::Device { action } => commands::device::run(action),
        Command::Doctor { json } => commands::doctor::run(json),
        Command::Setup { apply } => commands::setup::run(apply),
        Command::Config { action } => commands::config::run(action),
        Command::Module => {
            let _ = commands::status::run(false, true);
            Ok(())
        }
        Command::Compat(c) => run_compat(c),
    }
}

fn run_compat(cmd: CompatCommand) -> Result<()> {
    match cmd {
        CompatCommand::Mirror { device, profile } => {
            let config = hypr_phone::config::Config::load_default()?;
            let resolved = services::scrcpy::resolve_mirror_config(
                &config,
                services::scrcpy::MirrorRequest {
                    device_serial: device.as_deref(),
                    profile: profile.as_deref(),
                    ..Default::default()
                },
            )?;
            let args = services::scrcpy::build_scrcpy_args(&resolved);
            let bin = services::scrcpy::scrcpy_path()?;
            let mut c = StdCommand::new(bin);
            c.args(&args);
            let child = c.spawn()?;
            println!(
                "Started scrcpy (pid {}) with profile `{}`.",
                child.id(),
                resolved.profile_name
            );
            let _ = services::hyprland::place_window(&resolved.window_title, &resolved.placement);
            Ok(())
        }
        CompatCommand::Screenshot { target, output } => {
            commands::screenshot::run_compat(target, output)
        }
        CompatCommand::Push {
            local_path,
            remote_path,
            target,
        } => {
            let config = hypr_phone::config::Config::load_default()?;
            let serial = config.resolve_target_serial(target.as_deref());
            let out = services::adb::ops::push_file(serial.as_deref(), &local_path, &remote_path)?;
            println!("{out}");
            Ok(())
        }
        CompatCommand::Pull {
            remote_path,
            local_path,
            target,
        } => {
            let config = hypr_phone::config::Config::load_default()?;
            let serial = config.resolve_target_serial(target.as_deref());
            let out = services::adb::ops::pull_file(serial.as_deref(), &remote_path, &local_path)?;
            println!("{out}");
            Ok(())
        }
        CompatCommand::InstallApk {
            apk_path,
            target,
            reinstall,
        } => {
            let config = hypr_phone::config::Config::load_default()?;
            let serial = config.resolve_target_serial(target.as_deref());
            let out = services::adb::ops::install_apk(serial.as_deref(), &apk_path, reinstall)?;
            println!("{out}");
            Ok(())
        }
        CompatCommand::Shell { target, command } => {
            let config = hypr_phone::config::Config::load_default()?;
            let serial = config.resolve_target_serial(target.as_deref());
            let out = services::adb::ops::shell(serial.as_deref(), &command)?;
            println!("{out}");
            Ok(())
        }
        CompatCommand::Clipboard { action } => run_compat_clipboard(action),
        CompatCommand::Kde { action } => run_compat_kde(action),
        CompatCommand::Menu => commands::menu::run(),
        CompatCommand::GuiStatus => {
            let status = serde_json::json!({
                "config_version": hypr_phone::config::CURRENT_CONFIG_VERSION,
                "tray_enabled": false,
                "notes": "GUI scaffolding removed; use `hypr-phone setup` for first-run configuration."
            });
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
        CompatCommand::GuiRules => {
            println!("# hypr-phone is now runtime-driven. Recommended bindings:");
            println!("# bind = SUPER, P, exec, hypr-phone toggle");
            println!("# bind = SUPER SHIFT, P, exec, hypr-phone menu");
            println!("# bind = SUPER, A, exec, hypr-phone app");
            println!("# bind = SUPER SHIFT, S, exec, hypr-phone screenshot");
            Ok(())
        }
        CompatCommand::Devices { json } => {
            commands::device::run(hypr_phone::cli::DeviceAction::List { json })
        }
        CompatCommand::Pair {
            endpoint,
            pairing_code,
        } => commands::device::run(hypr_phone::cli::DeviceAction::Pair {
            endpoint,
            code: pairing_code,
        }),
        CompatCommand::Connect { endpoint } => {
            commands::device::run(hypr_phone::cli::DeviceAction::Connect { endpoint })
        }
        CompatCommand::Reconnect { target } => {
            commands::device::run(hypr_phone::cli::DeviceAction::Reconnect { target })
        }
        CompatCommand::Disconnect { serial } => {
            commands::device::run(hypr_phone::cli::DeviceAction::Disconnect { serial })
        }
    }
}

fn run_compat_clipboard(action: CompatClipboardAction) -> Result<()> {
    match action {
        CompatClipboardAction::Send { target, text } => {
            let config = hypr_phone::config::Config::load_default()?;
            let serial = config.resolve_target_serial(target.as_deref());
            let content = match text {
                Some(t) => t,
                None => services::clipboard::read().context("reading host clipboard")?,
            };
            let out = services::adb::ops::send_clipboard(serial.as_deref(), &content)?;
            println!("{out}");
            Ok(())
        }
        CompatClipboardAction::Receive { target } => {
            let config = hypr_phone::config::Config::load_default()?;
            let serial = config.resolve_target_serial(target.as_deref());
            let out = services::adb::ops::receive_clipboard(serial.as_deref())?;
            services::clipboard::write(&out)?;
            println!("{out}");
            Ok(())
        }
    }
}

fn run_compat_kde(action: CompatKdeAction) -> Result<()> {
    match action {
        CompatKdeAction::Devices => {
            let raw = services::kdeconnect::run_raw(&["--list-devices", "--id-name-only"])?;
            print!("{raw}");
            Ok(())
        }
        CompatKdeAction::Battery { target } => {
            let info = services::kdeconnect::battery(target.as_deref())?;
            println!(
                "Battery: {}%{}",
                info.level_percent,
                if info.charging { " (charging)" } else { "" }
            );
            Ok(())
        }
        CompatKdeAction::Ring { target } => {
            services::kdeconnect::ring(target.as_deref())?;
            println!("Ringing…");
            Ok(())
        }
        CompatKdeAction::Notify {
            target,
            title,
            body,
        } => {
            services::kdeconnect::notify(target.as_deref(), &title, &body)?;
            println!("Notification sent.");
            Ok(())
        }
        CompatKdeAction::Media { target, action } => {
            let mapped = match action {
                CompatMediaAction::Play => hypr_phone::cli::MediaAction::Play,
                CompatMediaAction::Pause => hypr_phone::cli::MediaAction::Pause,
                CompatMediaAction::PlayPause => hypr_phone::cli::MediaAction::PlayPause,
                CompatMediaAction::Stop => hypr_phone::cli::MediaAction::Stop,
                CompatMediaAction::Next => hypr_phone::cli::MediaAction::Next,
                CompatMediaAction::Previous => hypr_phone::cli::MediaAction::Previous,
            };
            services::kdeconnect::media_action(target.as_deref(), mapped)?;
            println!("Media action sent.");
            Ok(())
        }
    }
}

// Touch unused imports to keep them in main.rs.
#[allow(dead_code)]
fn _unused_imports() {
    let _: Result<()> = Err(anyhow!(""));
    let _ = io::stdout();
    let _ = ConfigAction::Path;
    let _ = ControlAction::Home;
    let _ = RecordAction::Status;
}

use std::{
    env,
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{bail, Context};

use crate::{
    adb,
    errors::Result,
    scrcpy::{DEFAULT_PROFILE_NAME, LOW_LATENCY_PROFILE_NAME},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuBackend {
    Rofi,
    Wofi,
}

impl MenuBackend {
    fn binary(self) -> &'static str {
        match self {
            MenuBackend::Rofi => "rofi",
            MenuBackend::Wofi => "wofi",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    MirrorDefaultDevice,
    MirrorLowLatency,
    ListDevices,
    PairWirelessAdb,
    ConnectWirelessAdb,
}

impl MenuAction {
    pub fn label(self) -> &'static str {
        match self {
            MenuAction::MirrorDefaultDevice => "Mirror default device",
            MenuAction::MirrorLowLatency => "Mirror low latency",
            MenuAction::ListDevices => "List devices",
            MenuAction::PairWirelessAdb => "Pair wireless ADB",
            MenuAction::ConnectWirelessAdb => "Connect wireless ADB",
        }
    }

    fn from_label(label: &str) -> Option<Self> {
        v01_actions()
            .into_iter()
            .find(|action| action.label() == label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub fn detect_menu_backend() -> Option<MenuBackend> {
    if which::which("rofi").is_ok() {
        return Some(MenuBackend::Rofi);
    }

    if which::which("wofi").is_ok() {
        return Some(MenuBackend::Wofi);
    }

    None
}

pub fn v01_actions() -> Vec<MenuAction> {
    vec![
        MenuAction::MirrorDefaultDevice,
        MenuAction::MirrorLowLatency,
        MenuAction::ListDevices,
        MenuAction::PairWirelessAdb,
        MenuAction::ConnectWirelessAdb,
    ]
}

pub fn launch_and_execute_v01_menu() -> Result<Option<String>> {
    let backend = detect_menu_backend().context("neither `rofi` nor `wofi` is available")?;

    let labels = v01_actions()
        .into_iter()
        .map(|action| action.label().to_string())
        .collect::<Vec<_>>();

    let Some(selection) = run_dmenu(backend, "hypr-phone", &labels)? else {
        return Ok(None);
    };

    let action = MenuAction::from_label(&selection)
        .with_context(|| format!("unknown menu action selected: {selection}"))?;

    let Some(command) = build_action_command(action, backend)? else {
        return Ok(None);
    };

    execute_action_command(&command).map(Some)
}

pub fn build_action_command(
    action: MenuAction,
    backend: MenuBackend,
) -> Result<Option<ActionCommand>> {
    match action {
        MenuAction::MirrorDefaultDevice => Ok(Some(build_hypr_phone_command(vec![
            "mirror".to_string(),
            "--profile".to_string(),
            DEFAULT_PROFILE_NAME.to_string(),
        ])?)),
        MenuAction::MirrorLowLatency => Ok(Some(build_hypr_phone_command(vec![
            "mirror".to_string(),
            "--profile".to_string(),
            LOW_LATENCY_PROFILE_NAME.to_string(),
        ])?)),
        MenuAction::ListDevices => Ok(Some(build_hypr_phone_command(vec!["devices".to_string()])?)),
        MenuAction::PairWirelessAdb => {
            let Some(endpoint) = prompt_endpoint(backend, "Pair endpoint (ip:port)", 37123)? else {
                return Ok(None);
            };
            let Some(code) = prompt_text(backend, "Pairing code")? else {
                return Ok(None);
            };

            Ok(Some(build_hypr_phone_command(vec![
                "pair".to_string(),
                endpoint,
                code,
            ])?))
        }
        MenuAction::ConnectWirelessAdb => {
            Ok(Some(build_hypr_phone_command(vec!["connect".to_string()])?))
        }
    }
}

pub fn execute_action_command(command: &ActionCommand) -> Result<String> {
    let output = Command::new(&command.program)
        .args(&command.args)
        .output()
        .with_context(|| format!("failed to execute `{}`", command.program))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        match (stdout.is_empty(), stderr.is_empty()) {
            (true, true) => Ok("ok".to_string()),
            (false, true) => Ok(stdout),
            (true, false) => Ok(stderr),
            (false, false) => Ok(format!("{stdout}\n{stderr}")),
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!(
            "command failed: {} {}{}",
            command.program,
            command.args.join(" "),
            if stderr.is_empty() {
                String::new()
            } else {
                format!("\n{stderr}")
            }
        )
    }
}

fn build_hypr_phone_command(args: Vec<String>) -> Result<ActionCommand> {
    let program = env::current_exe()
        .context("failed to resolve hypr-phone executable path for menu action")?;
    Ok(ActionCommand {
        program: program.to_string_lossy().into_owned(),
        args,
    })
}

fn prompt_endpoint(
    backend: MenuBackend,
    prompt: &str,
    default_port: u16,
) -> Result<Option<String>> {
    let Some(endpoint) = prompt_text(backend, prompt)? else {
        return Ok(None);
    };

    Ok(Some(adb::normalize_endpoint(&endpoint, default_port)))
}

fn prompt_text(backend: MenuBackend, prompt: &str) -> Result<Option<String>> {
    let Some(value) = run_dmenu(backend, prompt, &[])? else {
        return Ok(None);
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

fn run_dmenu(backend: MenuBackend, prompt: &str, entries: &[String]) -> Result<Option<String>> {
    let mut command = Command::new(backend.binary());
    match backend {
        MenuBackend::Rofi => {
            command.args(["-dmenu", "-i", "-p", prompt]);
        }
        MenuBackend::Wofi => {
            command.args(["--dmenu", "-i", "-p", prompt]);
        }
    }

    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to launch {}", backend.binary()))?;

    if let Some(mut stdin) = child.stdin.take() {
        if entries.is_empty() {
            stdin.write_all(b"\n")?;
        } else {
            for entry in entries {
                stdin.write_all(entry.as_bytes())?;
                stdin.write_all(b"\n")?;
            }
        }
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Ok(None);
    }

    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(selected))
    }
}

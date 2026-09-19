//! Integration tests for the hypr-phone CLI.
//!
//! These tests mock external binaries via PATH injection so we can exercise
//! the full CLI without real adb/scrcpy/hyprctl binaries.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).expect("write script");
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod");
}

fn stub_workspace_test(
    name: &str,
) -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    let config_root = tmp.path().join("config");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::create_dir_all(&config_root).unwrap();
    let adb_log = tmp.path().join("adb.log");
    let devices_output = tmp.path().join("adb-devices.txt");
    fs::write(
        &devices_output,
        "List of devices attached\n192.168.1.10:5555 device product:pixel model:Pixel_8\n",
    )
    .unwrap();
    write_test_config(&config_root);
    let _ = name;
    write_executable(
        &bin_dir.join("adb"),
        &format!(
            r#"#!/bin/bash
set -euo pipefail
echo "$@" >> "{}"
args=("$@")
if [[ "${{args[0]}}" == "-s" ]]; then
  args=("${{args[@]:2}}")
fi
case "${{args[0]:-}}" in
  devices)
    cat "{}"
    ;;
  connect)
    echo "connected to ${{args[1]:-unknown}}"
    ;;
  pair)
    echo "paired to ${{args[1]:-unknown}}"
    ;;
  exec-out)
    printf '\x89PNG\r\n\x1a\nmock-png'
    ;;
  shell)
    if [[ "${{args[*]}}" == *"cmd clipboard get-text"* ]]; then
      echo "phone-clipboard-value"
    else
      echo "shell-ok"
    fi
    ;;
  push)
    echo "pushed"
    ;;
  pull)
    echo "pulled"
    ;;
  install)
    echo "installed"
    ;;
  *)
    echo "ok"
    ;;
esac
"#,
            adb_log.display(),
            devices_output.display()
        ),
    );
    let _ = name;
    (
        tmp,
        bin_dir,
        adb_log,
        devices_output,
        config_root,
    )
}

fn write_test_config(config_root: &Path) {
    let cfg_dir = config_root.join("hypr-phone");
    fs::create_dir_all(&cfg_dir).expect("create config dir");
    fs::write(
        cfg_dir.join("config.toml"),
        r#"
config_version = 2

[mirror]
profile = "default"
default_alias = "pixel"

[devices.entries.pixel]
adb_serial = "192.168.1.10:5555"
adb_endpoint = "192.168.1.10:5555"
kdeconnect_id = "kde-pixel"

[reconnect]
auto_save_history = true
max_history = 10
"#,
    )
    .expect("write config");
}

fn run_hypr_phone(
    args: &[&str],
    bin_dir: &Path,
    config_root: &Path,
) -> std::process::Output {
    let binary = env!("CARGO_BIN_EXE_hypr-phone");
    let current_path = std::env::var("PATH").unwrap_or_default();
    Command::new(binary)
        .args(args)
        .env("PATH", format!("{}:{}", bin_dir.display(), current_path))
        .env("XDG_CONFIG_HOME", config_root)
        .output()
        .expect("execute hypr-phone")
}

#[test]
fn device_list_uses_adb_and_aliases() {
    let (tmp, bin_dir, _adb_log, _, config_root) = stub_workspace_test("device-list");
    let _ = tmp;

    let out = run_hypr_phone(&["device", "list"], &bin_dir, &config_root);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("192.168.1.10:5555"));
    assert!(stdout.contains("Pixel 8"));
}

#[test]
fn device_reconnect_uses_alias_endpoint() {
    let (tmp, bin_dir, adb_log, _, config_root) = stub_workspace_test("device-reconnect");
    let _ = tmp;
    let out = run_hypr_phone(&["device", "reconnect", "pixel"], &bin_dir, &config_root);
    assert!(out.status.success());
    let calls = fs::read_to_string(&adb_log).expect("read adb log");
    assert!(calls.contains("connect 192.168.1.10:5555"));
}

#[test]
fn clipboard_send_escapes_single_quotes_for_remote_shell() {
    let (tmp, bin_dir, adb_log, _, config_root) = stub_workspace_test("clipboard-quote");
    let _ = tmp;
    let out = run_hypr_phone(
        &[
            "compat",
            "clipboard",
            "send",
            "--target",
            "pixel",
            "--text",
            "can't break",
        ],
        &bin_dir,
        &config_root,
    );
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let calls = fs::read_to_string(&adb_log).expect("read adb log");
    assert!(calls.contains("cmd clipboard set-text 'can'\\''t break'"));
}

#[test]
fn screenshot_writes_png_to_target_path() {
    let (tmp, bin_dir, _adb_log, _, config_root) = stub_workspace_test("screenshot");
    let _ = tmp;
    let out_path = tmp.path().join("shot.png");
    let out = run_hypr_phone(
        &[
            "screenshot",
            "--output",
            out_path.to_string_lossy().as_ref(),
        ],
        &bin_dir,
        &config_root,
    );
    assert!(out.status.success());
    let bytes = fs::read(&out_path).expect("read screenshot");
    assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
}

#[test]
fn config_init_creates_default_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let config_root = tmp.path().join("config");
    fs::create_dir_all(&config_root).unwrap();

    let out = run_hypr_phone(&["config", "init"], &bin_dir, &config_root);
    assert!(out.status.success());
    let path = config_root.join("hypr-phone").join("config.toml");
    assert!(path.exists());
}

#[test]
fn doctor_reports_missing_dependencies() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("empty-bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let config_root = tmp.path().join("config");
    fs::create_dir_all(&config_root).unwrap();
    let path_only = bin_dir.to_string_lossy().to_string();

    let binary = env!("CARGO_BIN_EXE_hypr-phone");
    let out = Command::new(binary)
        .args(["doctor"])
        .env("PATH", &path_only)
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("doctor runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("[BROKEN]") || stdout.contains("[WARN]"));
}

#[test]
fn doctor_json_emits_machine_readable_report() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let bin_dir = tmp.path().join("empty-bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let config_root = tmp.path().join("config");
    fs::create_dir_all(&config_root).unwrap();

    let binary = env!("CARGO_BIN_EXE_hypr-phone");
    let out = Command::new(binary)
        .args(["doctor", "--json"])
        .env("PATH", bin_dir.to_string_lossy().to_string())
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("doctor json runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let _json: serde_json::Value = serde_json::from_str(&stdout).expect("parse JSON");
}

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn unique_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let dir = std::env::current_dir()
        .expect("cwd")
        .join("target")
        .join("test-artifacts")
        .join(format!("{name}-{stamp}"));
    fs::create_dir_all(&dir).expect("create test dir");
    dir
}

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).expect("write script");
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod");
}

fn prepare_adb_script(bin_dir: &Path, log_path: &Path, devices_path: &Path) {
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
    if [[ "${{args[*]}}" == *"cmd clipboard get text"* ]]; then
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
            log_path.display(),
            devices_path.display()
        ),
    );
}

fn prepare_clipboard_scripts(bin_dir: &Path, host_clipboard_file: &Path) {
    write_executable(
        &bin_dir.join("wl-paste"),
        &format!(
            r#"#!/bin/bash
set -euo pipefail
cat "{}"
"#,
            host_clipboard_file.display()
        ),
    );

    write_executable(
        &bin_dir.join("wl-copy"),
        &format!(
            r#"#!/bin/bash
set -euo pipefail
cat > "{}"
"#,
            host_clipboard_file.display()
        ),
    );
}

fn prepare_kdeconnect_script(bin_dir: &Path, log_path: &Path) {
    write_executable(
        &bin_dir.join("kdeconnect-cli"),
        &format!(
            r#"#!/bin/bash
set -euo pipefail
echo "$@" >> "{}"
if [[ "$*" == *"--battery"* ]]; then
  echo "Battery: 92%"
elif [[ "$*" == *"--list-devices"* ]]; then
  echo "kde-pixel Pixel 8"
else
  echo "kde-ok"
fi
"#,
            log_path.display()
        ),
    );
}

fn write_test_config(config_root: &Path) {
    let cfg_dir = config_root.join("hypr-phone");
    fs::create_dir_all(&cfg_dir).expect("create config dir");
    fs::write(
        cfg_dir.join("config.toml"),
        r#"
config_version = 1

[devices.aliases.pixel]
adb_serial = "SERIAL-PIXEL"
adb_endpoint = "192.168.1.50:5555"
kdeconnect_id = "kde-pixel"

[reconnect]
auto_save_history = true
max_history = 10
recent_endpoints = ["192.168.1.99:5555"]
"#,
    )
    .expect("write config");
}

fn run_hypr_phone(args: &[&str], bin_dir: &Path, config_root: &Path) -> std::process::Output {
    let binary = env!("CARGO_BIN_EXE_hypr-phone");
    let current_path = std::env::var("PATH").unwrap_or_default();
    Command::new(binary)
        .args(args)
        .env("PATH", format!("{}:{}", bin_dir.display(), current_path))
        .env("XDG_CONFIG_HOME", config_root)
        .output()
        .expect("execute hypr-phone")
}

fn run_hypr_phone_path_only(
    args: &[&str],
    path_value: &str,
    config_root: &Path,
) -> std::process::Output {
    let binary = env!("CARGO_BIN_EXE_hypr-phone");
    Command::new(binary)
        .args(args)
        .env("PATH", path_value)
        .env("XDG_CONFIG_HOME", config_root)
        .output()
        .expect("execute hypr-phone")
}

#[test]
fn devices_and_reconnect_use_adb_and_aliases() {
    let root = unique_dir("devices-reconnect");
    let bin_dir = root.join("bin");
    let config_root = root.join("config");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    fs::create_dir_all(&config_root).expect("config root");

    let adb_log = root.join("adb.log");
    let devices_output = root.join("adb-devices.txt");
    fs::write(
        &devices_output,
        "List of devices attached\nSERIAL-PIXEL device product:pixel model:Pixel_8\n",
    )
    .expect("write devices output");

    write_test_config(&config_root);
    prepare_adb_script(&bin_dir, &adb_log, &devices_output);

    let out = run_hypr_phone(&["devices"], &bin_dir, &config_root);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("SERIAL-PIXEL (device)"));

    let reconnect = run_hypr_phone(&["reconnect", "pixel"], &bin_dir, &config_root);
    assert!(reconnect.status.success());
    let reconnect_out = String::from_utf8_lossy(&reconnect.stdout);
    assert!(reconnect_out.contains("connected to 192.168.1.50:5555"));

    let adb_calls = fs::read_to_string(&adb_log).expect("read adb log");
    assert!(adb_calls.contains("devices -l"));
    assert!(adb_calls.contains("connect 192.168.1.50:5555"));
}

#[test]
fn reconnect_uses_connect_history_not_pair_endpoint() {
    let root = unique_dir("reconnect-history");
    let bin_dir = root.join("bin");
    let config_root = root.join("config");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    fs::create_dir_all(&config_root).expect("config root");

    let adb_log = root.join("adb.log");
    let devices_output = root.join("adb-devices.txt");
    fs::write(
        &devices_output,
        "List of devices attached\nSERIAL-PIXEL device product:pixel model:Pixel_8\n",
    )
    .expect("write devices output");

    write_test_config(&config_root);
    prepare_adb_script(&bin_dir, &adb_log, &devices_output);

    let pair = run_hypr_phone(&["pair", "192.168.1.33:37123", "123456"], &bin_dir, &config_root);
    assert!(pair.status.success());

    let reconnect = run_hypr_phone(&["reconnect"], &bin_dir, &config_root);
    assert!(reconnect.status.success());

    let adb_calls = fs::read_to_string(&adb_log).expect("read adb log");
    assert!(adb_calls.contains("pair 192.168.1.33:37123 123456"));
    assert!(adb_calls.contains("connect 192.168.1.99:5555"));
    assert!(!adb_calls.contains("connect 192.168.1.33:37123"));
}

#[test]
fn reconnect_unknown_target_fails_without_history_fallback() {
    let root = unique_dir("reconnect-unknown-target");
    let bin_dir = root.join("bin");
    let config_root = root.join("config");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    fs::create_dir_all(&config_root).expect("config root");

    let adb_log = root.join("adb.log");
    let devices_output = root.join("adb-devices.txt");
    fs::write(
        &devices_output,
        "List of devices attached\nSERIAL-PIXEL device product:pixel model:Pixel_8\n",
    )
    .expect("write devices output");

    write_test_config(&config_root);
    prepare_adb_script(&bin_dir, &adb_log, &devices_output);

    let reconnect = run_hypr_phone(&["reconnect", "does-not-exist"], &bin_dir, &config_root);
    assert!(!reconnect.status.success());

    let stderr = String::from_utf8_lossy(&reconnect.stderr);
    assert!(stderr.contains("No known wireless endpoint found"));

    if adb_log.exists() {
        let adb_calls = fs::read_to_string(&adb_log).expect("read adb log");
        assert!(!adb_calls.contains("connect 192.168.1.99:5555"));
    }
}

#[test]
fn screenshot_push_pull_install_shell_and_clipboard_flow() {
    let root = unique_dir("v02-commands");
    let bin_dir = root.join("bin");
    let config_root = root.join("config");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    fs::create_dir_all(&config_root).expect("config root");

    let adb_log = root.join("adb.log");
    let devices_output = root.join("adb-devices.txt");
    fs::write(
        &devices_output,
        "List of devices attached\nSERIAL-PIXEL device product:pixel model:Pixel_8\n",
    )
    .expect("write devices output");

    let host_clipboard_file = root.join("host-clipboard.txt");
    fs::write(&host_clipboard_file, "desktop clipboard").expect("seed clipboard");

    write_test_config(&config_root);
    prepare_adb_script(&bin_dir, &adb_log, &devices_output);
    prepare_clipboard_scripts(&bin_dir, &host_clipboard_file);

    let screenshot_path = root.join("shot.png");
    let shot = run_hypr_phone(
        &[
            "screenshot",
            "pixel",
            screenshot_path.to_string_lossy().as_ref(),
        ],
        &bin_dir,
        &config_root,
    );
    assert!(shot.status.success());
    let bytes = fs::read(&screenshot_path).expect("read screenshot");
    assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));

    fs::write(root.join("local.txt"), "hello").expect("write local push file");
    let push = run_hypr_phone(
        &[
            "push",
            root.join("local.txt").to_string_lossy().as_ref(),
            "/sdcard/local.txt",
            "pixel",
        ],
        &bin_dir,
        &config_root,
    );
    assert!(push.status.success());

    let pull = run_hypr_phone(
        &[
            "pull",
            "/sdcard/remote.txt",
            root.join("remote.txt").to_string_lossy().as_ref(),
            "pixel",
        ],
        &bin_dir,
        &config_root,
    );
    assert!(pull.status.success());

    let install = run_hypr_phone(
        &["install-apk", "app.apk", "pixel", "--reinstall"],
        &bin_dir,
        &config_root,
    );
    assert!(install.status.success());

    let shell = run_hypr_phone(
        &["shell", "--target", "pixel", "echo", "hello"],
        &bin_dir,
        &config_root,
    );
    assert!(shell.status.success());

    let clip_send = run_hypr_phone(
        &[
            "clipboard",
            "send",
            "--target",
            "pixel",
            "--text",
            "from-host",
        ],
        &bin_dir,
        &config_root,
    );
    assert!(clip_send.status.success());

    let clip_recv = run_hypr_phone(
        &["clipboard", "receive", "--target", "pixel"],
        &bin_dir,
        &config_root,
    );
    assert!(clip_recv.status.success());
    let host_clipboard = fs::read_to_string(&host_clipboard_file).expect("read clipboard file");
    assert!(host_clipboard.contains("phone-clipboard-value"));

    let adb_calls = fs::read_to_string(&adb_log).expect("read adb log");
    assert!(adb_calls.contains("-s SERIAL-PIXEL push"));
    assert!(adb_calls.contains("-s SERIAL-PIXEL pull"));
    assert!(adb_calls.contains("-s SERIAL-PIXEL install -r app.apk"));
    assert!(adb_calls.contains("-s SERIAL-PIXEL shell echo hello"));
    assert!(adb_calls.contains("-s SERIAL-PIXEL shell cmd clipboard set text from-host"));
}

#[test]
fn clipboard_send_keeps_special_characters_literal() {
    let root = unique_dir("clipboard-special-chars");
    let bin_dir = root.join("bin");
    let config_root = root.join("config");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    fs::create_dir_all(&config_root).expect("config root");

    let adb_log = root.join("adb.log");
    let devices_output = root.join("adb-devices.txt");
    fs::write(
        &devices_output,
        "List of devices attached\nSERIAL-PIXEL device product:pixel model:Pixel_8\n",
    )
    .expect("write devices output");

    write_test_config(&config_root);
    prepare_adb_script(&bin_dir, &adb_log, &devices_output);

    let payload = "safe$(uname)`id`";
    let output = run_hypr_phone(
        &[
            "clipboard",
            "send",
            "--target",
            "pixel",
            "--text",
            payload,
        ],
        &bin_dir,
        &config_root,
    );
    assert!(output.status.success());

    let adb_calls = fs::read_to_string(&adb_log).expect("read adb log");
    assert!(adb_calls.contains("-s SERIAL-PIXEL shell cmd clipboard set text safe$(uname)`id`"));
}

#[test]
fn kde_bridge_uses_alias_and_graceful_fallback() {
    let root = unique_dir("kde-bridge");
    let bin_dir = root.join("bin");
    let config_root = root.join("config");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    fs::create_dir_all(&config_root).expect("config root");

    let adb_log = root.join("adb.log");
    let kde_log = root.join("kde.log");
    let devices_output = root.join("adb-devices.txt");
    fs::write(
        &devices_output,
        "List of devices attached\nSERIAL-PIXEL device product:pixel model:Pixel_8\n",
    )
    .expect("write devices output");

    write_test_config(&config_root);
    prepare_adb_script(&bin_dir, &adb_log, &devices_output);
    prepare_kdeconnect_script(&bin_dir, &kde_log);

    let battery = run_hypr_phone(
        &["kde", "battery", "--target", "pixel"],
        &bin_dir,
        &config_root,
    );
    assert!(battery.status.success());
    let out = String::from_utf8_lossy(&battery.stdout);
    assert!(out.contains("Battery: 92%"));

    let media = run_hypr_phone(
        &["kde", "media", "--target", "pixel", "play"],
        &bin_dir,
        &config_root,
    );
    assert!(media.status.success());

    let kde_calls = fs::read_to_string(&kde_log).expect("read kde log");
    assert!(kde_calls.contains("--device kde-pixel --battery"));
    assert!(kde_calls.contains("--device kde-pixel --mpris play"));

    fs::remove_file(bin_dir.join("kdeconnect-cli")).expect("remove kde script");
    let missing = run_hypr_phone_path_only(
        &["kde", "devices"],
        &bin_dir.to_string_lossy(),
        &config_root,
    );
    assert!(!missing.status.success());
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert!(stderr.contains("KDE Connect bridge unavailable"));
}

#[test]
fn gui_and_config_status_commands_expose_scaffolding() {
    let root = unique_dir("gui-config-status");
    let bin_dir = root.join("bin");
    let config_root = root.join("config");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    fs::create_dir_all(&config_root).expect("config root");

    let adb_log = root.join("adb.log");
    let devices_output = root.join("adb-devices.txt");
    fs::write(&devices_output, "List of devices attached\n").expect("write devices output");

    write_test_config(&config_root);
    prepare_adb_script(&bin_dir, &adb_log, &devices_output);

    let config_status = run_hypr_phone(&["config", "status"], &bin_dir, &config_root);
    assert!(config_status.status.success());
    let status = String::from_utf8_lossy(&config_status.stdout);
    assert!(status.contains("config_version = 1"));
    assert!(status.contains("stable_format = yes"));

    let gui_status = run_hypr_phone(&["gui", "status"], &bin_dir, &config_root);
    assert!(gui_status.status.success());
    let gui_json = String::from_utf8_lossy(&gui_status.stdout);
    assert!(gui_json.contains("\"config_version\": 1"));
    assert!(gui_json.contains("\"tray_enabled\": true"));

    let rules = run_hypr_phone(&["gui", "generate-rules"], &bin_dir, &config_root);
    assert!(rules.status.success());
    let rules_out = String::from_utf8_lossy(&rules.stdout);
    assert!(rules_out.contains("windowrulev2 = workspace special:phone"));
}

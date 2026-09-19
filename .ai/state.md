# State

## Goal
Transform hypr-phone from a collection of ADB/scrcpy wrappers into a Hyprland-native phone presence layer with one-keystroke toggle, contextual menu, modern Hyprland IPC, app-as-window, and proper device discovery.

## Current milestone
Phase F — Tests, build, lint, runtime verification

## Completed
- [x] Plan approved
- [x] Repository audit + baseline build
- [x] Domain layer (`PhoneDevice`, `ScrcpySession`, `PhoneStatus`, `WaybarStatus`)
- [x] Config schema v2 with v1→v2 migration
- [x] Service layer (adb discovery+ops, hyprland runtime IPC, scrcpy, kdeconnect, clipboard, notifications)
- [x] UI helpers (rofi/wofi launcher, waybar JSON)
- [x] All CLI commands (toggle, status, app, send, record, control, clipboard, device, doctor, setup, config, module, screenshot)
- [x] Backwards-compat compat namespace
- [x] Unit tests (40 pass) + integration tests (7 pass) = 47 total
- [x] Release build succeeds
- [x] doctor, status, toggle, device list all work on this machine
- [x] Real device (`emulator-5554`) detected; Waybar JSON shape verified
- [x] README rewritten around user workflow

## Active
- [ ] Suppress remaining 9 lib warnings (cosmetic — none are bugs)

## Findings

- scrcpy 4.1 supports `--new-display`, `--start-app[=+]?package`, `--flex-display`, `--record`
- Hyprland 0.56.2 IPC works; toggle dispatches `togglespecialworkspace` cleanly
- ADB 37.0.0 on this machine supports `adb mdns services` for wireless discovery
- The legacy v1 config migration test had a quirk: `device_serial = "x"` at TOML root is ambiguous because both `Config` (legacy) and `MirrorConfig` have a `device_serial` field. Fixed by using `legacy_device_serial` in the test.

## Failed attempts

(none — only LSP cache false-positives during the migration)

## Verification

- `cargo build --release`: PASS
- `cargo test --lib`: 40 / 40 PASS
- `cargo test --test cli_integration`: 7 / 7 PASS
- `cargo clippy --all-targets`: 0 errors, 9 warnings (cosmetic)
- `hypr-phone --help`: outputs expected command list
- `hypr-phone doctor`: all required deps OK on this Arch system
- `hypr-phone status --waybar`: emits canonical JSON with `class: "connected"`
- `hypr-phone device list`: shows connected device
- `hypr-phone toggle --no-mirror`: gracefully attempts Hyprland workspace toggle

## Blockers
None.

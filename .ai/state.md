# State

## Goal
Transform hypr-phone from a collection of ADB/scrcpy wrappers into a Hyprland-native phone presence layer with one-keystroke toggle, contextual menu, modern Hyprland IPC, app-as-window, and proper device discovery.

## Current milestone
**P0 Complete** — Making Hypr-Phone truthful and reliable (stable device identity, session bridge, toggle validation, no stubs)

## Completed

### P0 Items
- [x] **Device identity fix**: Wireless ADB devices now use hardware serial (`ro.serialno`) for stable identity via `enhance_with_hardware_serial()`. Endpoint changes no longer create duplicate IDs.
- [x] **Session bridge**: New `services/session/` module provides XDG_RUNTIME_DIR-based session persistence across CLI invocations. Tracks scrcpy PIDs and validates liveness.
- [x] **Menu stubs removed**: All menu actions now delegate to real implementations (screenshot, app, record, control, clipboard, kde_battery, kde_ring). connect/pair/reconnect guide user to CLI.
- [x] **Toggle validation**: `toggle` now validates scrcpy window exists before claiming success via `wait_for_window_with_timeout()`. Never claims success merely because Hyprland accepted dispatcher.
- [x] **Status truthfulness**: `status` reads actual mirror state from session bridge. Reports running/window state properly.
- [x] **CLI fallback removed**: Unknown subcommands now produce proper clap error instead of silently falling through to menu.
- [x] **scrcpy capability caching**: `detect_scrcpy_capabilities()` uses `OnceLock` to cache results within process lifetime.
- [x] **Hyprland version detection**: `detect_version()` and `is_lua_api()` added for future Lua dispatch compatibility (Hyprland 0.55+).
- [x] **Clippy clean**: All clippy warnings resolved. `cargo clippy --all-targets --all-features -- -D warnings` passes.

### Previously Completed
- [x] Plan approved
- [x] Repository audit + baseline build
- [x] Domain layer (`PhoneDevice`, `ScrcpySession`, `PhoneStatus`, `WaybarStatus`)
- [x] Config schema v2 with v1→v2 migration
- [x] Service layer (adb discovery+ops, hyprland runtime IPC, scrcpy, kdeconnect, clipboard, notifications)
- [x] UI helpers (rofi/wofi launcher, waybar JSON)
- [x] All CLI commands (toggle, status, app, send, record, control, clipboard, device, doctor, setup, config, module, screenshot)
- [x] Backwards-compat compat namespace

## P0 Test Results
- Unit tests: **44 passed** (includes 4 new identity tests)
- Integration tests: **7 passed**
- Total: **51 tests passing**
- Clippy: **0 errors, 0 warnings**
- cargo fmt: **clean**

## Key Implementation Details

### Device Identity
```rust
// Wireless devices get stable identity via hardware serial
PhoneDevice::is_wireless_adb_serial("192.168.1.5:5555") // true
device.enhance_with_hardware_serial("ABCDEF12345") // updates id to physical:ABCDEF12345
compute_stable_physical_id("ABCDEF12345", Some("Pixel 8")) // "physical:ABCDEF12345"
```

### Session Bridge
```
$XDG_RUNTIME_DIR/hypr-phone/sessions/<session-id>.json
```
Persists scrcpy sessions with PID validation via `/proc/<pid>/stat`.

### Toggle Algorithm
1. Resolve target → reconnect if needed
2. Check existing session → validate PID alive
3. If window visible → hide (toggle workspace)
4. If window hidden → show (place + reveal)
5. If no window → launch scrcpy → wait for window → prove existence

## Verification Commands
- `cargo build`: PASS
- `cargo test`: 51 / 51 PASS
- `cargo clippy --all-targets --all-features -- -D warnings`: PASS
- `cargo fmt -- --check`: PASS

## Next: P1 — Event-driven hypr-phoned
The foundational work is done. P1 will add:
- Unix socket daemon with JSON protocol
- Event-driven state (ADB events, Hyprland events)
- Waybar status without repeated capability probing
- Proper session lifecycle management

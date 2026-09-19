# Hypr Phone — Implementation Plan v0.4/v1.0 Rethink

## Status snapshot

I began executing the approved plan but must pause because plan mode was
re-asserted. The current repository state is partially migrated:

- **New domain layer added**: `src/domain/{device,session,status,error}.rs`
- **New config schema (v2) added**: `src/config/{mod,schema}.rs` with proper
  device-identity + transport separation and v1→v2 migration
- **New service modules**: `src/services/{adb,hyprland,scrcpy,kdeconnect,
  clipboard,notifications}.rs` (adb split into discovery+ops submodules)
- **New UI modules**: `src/ui/{rofi,waybar}.rs`
- **New command stubs**: `src/commands/{mod,device,status,toggle,menu}.rs`
- **Old top-level modules remain**: `src/{adb,cli,config,gui,hyprland,
  kdeconnect,menu,module_output,scrcpy,errors,main,lib}.rs` — they conflict
  with the new layout (e.g. duplicate `config`, `hyprland`, `menu`)
- **Build does not compile** in this interim state — `cargo check` fails
  because `lib.rs` declares `pub mod services;` etc. but the old
  `src/adb.rs`, `src/scrcpy.rs` etc. still exist and shadow the new layout

The repository needs either to **complete the migration** (remove the old
top-level modules and rewrite `main.rs` + `cli.rs` to use the new
structure) or to **discard the partial work and start fresh**. Given the
work is on track and already extensive, the recommendation is to complete
it.

---

## OBSERVED — Original Repository State

### What existed before the migration

- **Stack**: Rust 2021, flat module structure, ~8 source modules in `src/`
- **CLI**: ~15 top-level commands (mirror, screenshot, push, pull,
  install-apk, shell, clipboard, kde, module, menu, config, gui, doctor,
  pair, connect, reconnect, disconnect, devices)
- **Device model**: `AdbDevice` with serial+state+model+product;
  `DeviceAlias` stored `adb_serial, adb_endpoint, kdeconnect_id` but
  endpoint-as-string was effectively the identity
- **Config**: TOML, versioned (v0→v1), with aliases, profiles, reconnect
  history
- **Hyprland**: generated legacy `windowrulev2` config + runtime
  `hyprctl dispatch`; Hyprland 0.56.2 at `/usr/bin/hyprctl` in this env
- **Menu**: Static flat action list, not contextual
- **Status**: `ModuleStatus` JSON for Waybar, calls `adb devices` every
  invocation
- **Doctor**: Binary-presence only, no version/capability checks
- **Tests**: 7 integration tests covering devices, reconnect, clipboard,
  kde, gui/config scaffolding

### What's broken / missing (vs. the redesign brief)

1. **No `toggle`** — no single command that discovers → connects →
   mirrors → places → focuses
2. **IP-as-identity** — `DeviceAlias.adb_endpoint` was just a stored
   `String`; endpoint changes broke device selection
3. **No mDNS/ADB Wi-Fi discovery** — only `adb devices` polling
4. **Static menu** — showed all actions regardless of connection state
5. **No app-as-window** — scrcpy `--new-display --start-app` was not
   exposed at all
6. **No daemon** — every invocation spawned fresh `adb devices`; no
   cached device list
7. **Legacy Hyprland rules** — generated `windowrulev2` config;
   Hyprland 0.56.2 prefers runtime IPC
8. **Doctor is shallow** — no version checks, no capability detection,
   no fix suggestions
9. **No `send`** — no intelligent file/URL/text routing
10. **No recording** — screen recording not implemented
11. **No `setup`** — no guided first-run experience
12. **Giant `main.rs`** — 655 lines, all command dispatch and business
    logic co-mingled

---

## User Outcome

> `SUPER+P` → phone appears. `SUPER+SHIFT+P` → contextual action menu.
> No IPs typed. No adb commands run manually. The phone feels like a
> Hyprland-native device.

---

## Acceptance Criteria

### P0 — Daily-use core

- [ ] `hypr-phone toggle` reliably discovers, connects, mirrors, and
      places phone via `special:phone`
- [ ] `hypr-phone toggle` when already mirrored → toggles workspace
      visibility
- [ ] `hypr-phone toggle` recovers gracefully when scrcpy process dies
- [ ] `hypr-phone` (no args) opens a contextual menu: connected vs.
      disconnected state shows different actions
- [ ] Proper `PhoneDevice` model: identity separate from transport;
      supports USB, Wi-Fi, mDNS-discovered
- [ ] Device selection: configured default + single device auto-selected
      + multi-device picker only when genuinely ambiguous
- [ ] Canonical `PhoneStatus` consumed by CLI output, menu, Waybar
- [ ] Modern Hyprland IPC placement via `hyprctl` dispatch, not
      `windowrulev2` generation
- [ ] `hypr-phone doctor --json` becomes a thorough diagnostic tool
- [ ] All existing commands continue to work (backwards compatibility)

### P1 — Differentiating workflows

- [ ] `hypr-phone app` lists Android apps and launches via
      `--new-display --start-app`
- [ ] `hypr-phone send <file|url|text>` routes intelligently (KDE
      Connect → ADB fallback)
- [ ] `hypr-phone screenshot` defaults to `~/Pictures/hypr-phone/` with
      notification
- [ ] `hypr-phone record` with proper lifecycle (start/stop/status, no
      duplicates)
- [ ] `hypr-phone status --waybar` / `module` outputs richer canonical
      JSON with battery, connection type, KDE Connect
- [ ] `hypr-phone setup` guided first-run experience

### P2 — Polish

- [ ] Daemon (`hypr-phoned`) for cached state, event-driven device
      tracking, fast status
- [ ] Architecture refactor: domain/services/commands/ui separation
- [ ] Shell completions + man page
- [ ] README rewritten around user workflow, not command inventory

---

## Implementation Sequence (next steps from current state)

### Phase A — Finish the migration

**Affected files**: `src/lib.rs`, `src/main.rs`, `src/cli.rs`,
delete `src/{adb,cli,config,gui,hyprland,kdeconnect,menu,module_output,
scrcpy,errors}.rs`

1. **Rewrite `src/lib.rs`** to declare the new modules (already done in
   the partial work — needs verification that all referenced modules
   exist).

2. **Rewrite `src/cli.rs`** to add new commands: `toggle`, `status`,
   `app`, `send`, `record`, `device` (with `DeviceCommand` enum),
   `doctor --json`, `setup`.

3. **Rewrite `src/main.rs`** to be a thin dispatch table (~80 lines):
   - Parse `Cli`
   - Route to `commands::toggle::run` / `commands::status::run` /
     `commands::doctor::run` / etc.
   - No business logic in main

4. **Remove obsolete top-level files**:
   `src/{adb,config,gui,hyprland,kdeconnect,menu,module_output,scrcpy,
   errors}.rs` are now superseded. Keep nothing — or migrate any
   remaining useful code (e.g. `gui::generate_hyprland_rules`) into the
   new layout under `services/hyprland`.

5. **Verify** `cargo check` succeeds and `cargo test --lib` passes.

**Risk**: integration test file `tests/cli_integration.rs` references old
binary args; needs to be updated to the new command surface.

---

### Phase B — Complete the device commands

**Affected files**: `src/commands/{device,menu}.rs`,
`src/services/scrcpy.rs`, `src/ui/rofi.rs`

1. **Replace `commands/menu.rs` stub** with a real implementation that
   builds `MenuContext`, queries `PhoneDevice`s via `AdbDiscovery`,
   constructs contextual actions, and dispatches via rofi/wofi.
2. **Replace `commands/device.rs` stub** with proper dispatch using the
   new `DeviceCommand` CLI enum (list, pair, connect, reconnect,
   disconnect).
3. **Wire `services::scrcpy::spawn_mirror`** through `commands::toggle`
   to actually launch scrcpy.
4. **Add `commands/app.rs`** implementing `hypr-phone app [<pkg>]`:
   - List apps via `scrcpy --list-apps` (parse output)
   - Fuzzy search via rofi
   - Launch with `--new-display --start-app=+pkg`

---

### Phase C — Recording, send, doctor, setup

**Affected files**: new `src/commands/{record,send,doctor,setup}.rs`,
`src/commands/screenshot.rs` (or fold into `send`)

1. **`commands/record.rs`**:
   - `hypr-phone record [start|stop|status]`
   - Default output: `$XDG_VIDEOS_DIR/hypr-phone/<device>-<timestamp>.mp4`
   - Use `scrcpy::build_scrcpy_record_args`
   - Session manager prevents duplicates

2. **`commands/send.rs`**:
   - `hypr-phone send <path|url|text>`
   - Detect by content: existing file → push; `http(s)://` → am start;
     else → clipboard
   - Prefer KDE Connect if paired, fallback to ADB

3. **`commands/doctor.rs`**:
   - Detect adb/scrcpy/hyprctl/rofi-wofi/notify-send/kdeconnect
   - Parse versions from `--version` / `--help`
   - Capability detection (scrcpy::detect_scrcpy_capabilities)
   - Print `[OK]` / `[WARN]` / `[BROKEN]` / `[HINT]` lines
   - `--json` flag

4. **`commands/setup.rs`**:
   - `hypr-phone setup [--apply]`
   - Step through: detect tools → list devices → write config
     (`config init`) → emit suggested Hyprland bindings (printed, not
     written)
   - Never modify user files

---

### Phase D — Backwards compatibility + CLI surface

**Affected files**: `src/cli.rs`, `src/commands/config.rs`

1. **Keep aliases** for old commands: `mirror`, `screenshot`, `push`,
   `pull`, `install-apk`, `shell`, `clipboard`, `kde`, `devices`,
   `pair`, `connect`, `reconnect`, `disconnect`, `module`, `menu`,
   `config`, `gui`.
2. **Add new commands**: `toggle`, `status [--json] [--waybar]`, `app
   [<pkg>]`, `send <thing>`, `record [start|stop|status]`, `doctor
   [--json]`, `setup [--apply]`, `device {list|pair|connect|reconnect|
   disconnect}`.
3. **`commands/config.rs`**: implement `path`, `init`, `status`,
   `migrate` (already mostly there in v1).

---

### Phase E — Tests

1. **Domain unit tests** — already in `domain/{device,session,status}.rs`
2. **Config migration tests** — already in `config/schema.rs`
3. **Scrcpy arg generation** — already in `services/scrcpy.rs`
4. **Update `tests/cli_integration.rs`** to exercise:
   - `toggle` cycle (mocked `scrcpy`/`hyprctl`)
   - `device list --json` parses JSON
   - `status --waybar` JSON shape
   - `app` argument generation
5. **Add unit tests for menu action selection** per connection state.

---

### Phase F — Build, lint, runtime

1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo build --release`
4. `cargo test --all`
5. Run `hypr-phone --help`, `hypr-phone doctor`, `hypr-phone status`
6. If Hyprland running, exercise `hypr-phone toggle`
7. If real device available, exercise end-to-end discovery + mirror

---

### Phase G — Documentation

1. **Rewrite README** around the user workflow:
   - 60-second setup
   - `SUPER+P` daily workflow
   - Contextual menu
   - Waybar integration example
   - Android apps as windows
2. **Update `docs/`** with new commands and behavior
3. **Add `docs/architecture.md`** explaining domain/services/commands/ui

---

## Key Decisions (unchanged from original plan)

| Question | Decision | Rationale |
|---|---|---|
| Daemon or stateless? | Optional user daemon (`hypr-phoned`) on-demand; CLI degrades gracefully without it | Fast status for Waybar, event-driven discovery; not required for basic operation |
| Device identity | Stable `DeviceId` (hash of serial+model) not IP | Endpoint changes don't break device selection |
| Hyprland config | Runtime IPC via socket first, hyprctl fallback | Hyprland 0.56.2 has stable socket API; config generation only for users who want static rules |
| Menu backend | rofi → wofi → error (no custom TUI) | Already supported; no new dependency |
| scrcpy abstraction | Profiles + raw args escape hatch | Don't reimplement scrcpy; give sensible presets |
| Virtual display | `--new-display --start-app`; degrade if unsupported | scrcpy 4.1 supports it; feature-detect |
| Recording | `scrcpy --record` orchestration | scrcpy handles it |

---

## Non-goals (explicit)

- GUI/Tauri tray (retain scaffolding notes, not implementation)
- Replace scrcpy or KDE Connect
- nmap LAN scanning
- Network daemon (Unix socket only, local user scope)
- Telemetry or cloud
- Noctalia/Quickshell/AGS dependency

---

## Failure modes to handle

1. **Phone not reachable**: `toggle` attempts reconnect (up to 2 tries,
   3s each) before asking user
2. **scrcpy dies mid-session**: `toggle` detects dead process, offers to
   relaunch
3. **Endpoint changed**: device identity via serial/model, not stored IP;
   auto-rediscover
4. **Multiple devices, no default**: prompt for device selection only
   in this case
5. **KDE Connect unavailable**: hide KDE features, core ADB+scrcpy
   still works
6. **Hyprland not running**: graceful error, don't crash

---

## Stop condition

The implementation is complete when:
1. `cargo test --all` + `cargo clippy --all-targets -- -D warnings` pass
2. `hypr-phone toggle` discovers a real device, launches scrcpy, places
   it in `special:phone`
3. `hypr-phone` (no args) shows contextual menu reflecting actual
   connection state
4. `hypr-phone status --waybar` outputs valid JSON with `class` and
   `tooltip`
5. `hypr-phone app` lists real installed apps from a connected device
6. README accurately describes what actually exists

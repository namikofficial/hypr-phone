# Hypr-Phone — Final Master Implementation Plan

> **Status:** implementation-ready master plan  
> **Last research pass:** 2026-09-19  
> **Repository:** `namikofficial/hypr-phone`  
> **Primary platform:** Arch Linux + Hyprland + Wayland  
> **Primary developer target:** x86_64 Linux laptop with Intel CPU/KVM and NVIDIA dGPU  
> **Primary Android workflows:** physical Pixel + Android Emulator + Expo/React Native development

---

## 0. Final product decision

Hypr-Phone should **not** become “scrcpy plus AVD-SLIM”. It should become:

> **The Android control plane for Hyprland — physical phones, Android emulators, Android apps, and developer workflows as native desktop citizens.**

The daily contract is deliberately tiny:

> **Press `SUPER+P`; Android appears.**

Everything else exists to make that contract truthful.

`SUPER+P` must automatically do the correct thing:

- selected physical phone connected → reveal/focus its mirror;
- Wi-Fi ADB dropped → reconnect, then reveal;
- scrcpy process died → relaunch it;
- selected AVD stopped → boot it, wait until Android is ready, then reveal it;
- existing Android window hidden → reveal it;
- existing Android window visible → hide it;
- target unavailable → show a useful target/recovery picker;
- multiple valid targets with no selected default → ask once, remember the choice;
- no Android tooling installed → show an actionable diagnosis instead of failing silently.

The product is successful when Hypr-Phone becomes the fastest path to:

1. use the real phone from Hyprland;
2. start a lightweight Android development environment;
3. run an Android app as a desktop window;
4. build/install/debug a mobile project without opening Android Studio;
5. understand exactly what Android-related RAM/disk/CPU is being consumed.

Do **not** optimize for the number of commands. Optimize for **friction removed per interaction**.

---

# 1. Current repository reality

Do not trust the existing README or `.ai/state.md` as proof of implementation. Current source must remain the authority.

The current tree has a good architectural direction—domain/services/commands/UI separation, ADB discovery, scrcpy profiles, Hyprland placement, KDE Connect hooks—but important daily paths are still incomplete or semantically wrong.

| Area | Current source reality | Why it matters |
|---|---|---|
| Contextual menu | `src/commands/menu.rs` still dispatches many entries to literal `(stub)` output | The default UI is not actually a control plane |
| `toggle` | `src/commands/toggle.rs` can call `togglespecialworkspace` and return without proving a scrcpy window exists | `SUPER+P` can reveal an empty special workspace |
| Session state | `status.mirror = None` is hardcoded | Status cannot truthfully say whether mirroring is alive |
| Session manager | Existing `ScrcpySessionManager` is process-local/in-memory only | Every CLI invocation loses all session knowledge |
| Device identity | Current `compute_device_id()` includes the ADB serial; Wi-Fi ADB serials are endpoints | A device changing IP/port can get a different identity despite comments claiming stability |
| AVDs | `emulator-5554` is just another ADB row | No AVD lifecycle, profile, snapshot, storage, boot or tuning ownership |
| Status path | Repeated ADB/scrcpy capability probing | Bad Waybar polling behavior and needless latency |
| Hyprland placement | Multiple synchronous `hyprctl` calls operate on active/focused state | Race-prone and can affect the wrong window |
| Hyprland version | Current project logic assumes legacy dispatcher syntax | Hyprland 0.55+ moved toward Lua dispatch APIs; version adaptation is required |
| App picker | Mostly package IDs from `pm list packages -3` | Not good enough to replace phone-side launching |
| Unknown subcommands | External commands fall back into the menu | Typos get hidden instead of diagnosed |
| Docs/state | Historical state can claim completion while code still has stubs | Future AI agents can make incorrect assumptions |

### First code bug to fix: device identity

The current code says device identity is stable across endpoint changes, but this implementation is not:

```rust
compute_device_id(serial, model)
```

For wireless ADB, `serial` may be `192.168.x.x:port`, so the ID changes when the endpoint changes.

The permanent model must distinguish:

- **stable identity**: hardware serial / `ro.serialno` / configured persistent ID;
- **ADB serial**: current connection handle;
- **transport**: USB, Wi-Fi, mDNS, emulator;
- **endpoint**: transient address;
- **target type**: physical / AVD / Waydroid.

Do not build the daemon or AVD layer on top of the current endpoint-derived ID.

---

# 2. Decisions already made

These are not open-ended design questions. Implement them unless runtime evidence proves one impossible.

## 2.1 Keep Rust

Keep the current Rust CLI/service architecture. Do not rewrite the project in Python, Go, TypeScript, Tauri, Electron, AGS, Quickshell, or shell scripts.

The project needs:

- fast process startup;
- reliable process ownership;
- Unix sockets;
- long-running event subscriptions;
- typed state;
- low idle memory;
- simple single-binary distribution.

Rust already fits.

## 2.2 Linux/Hyprland first

Do not spend architecture budget on Windows/macOS support. Hypr-Phone is intentionally a Hyprland-native Linux tool.

Portable abstractions are welcome where free, but do not weaken the Linux implementation for theoretical portability.

## 2.3 `hypr-phoned` becomes core, not P7 polish

The CLI should eventually be a client of a small user daemon.

The daemon owns:

- discovered Android targets;
- target selection;
- ADB reachability;
- AVD lifecycle;
- child process lifecycle;
- scrcpy sessions;
- Hyprland window addresses;
- app metadata cache;
- capability cache;
- boot state;
- measurements;
- last errors;
- state sequence number / freshness timestamp.

The daemon must be optional during migration, but once P1 is complete it becomes the normal path.

## 2.4 Use a single `AndroidTarget` abstraction

Three backend types:

1. `Physical` — USB/Wi-Fi/mDNS Android device;
2. `Avd` — official Android Emulator virtual device;
3. `Waydroid` — optional later backend.

Shared operations should not duplicate logic by backend:

- shell;
- APK install;
- screenshot;
- app launch;
- package clear;
- logcat;
- port reverse;
- deep link / intent;
- clipboard where capability exists;
- recording where capability exists.

Backend-specific lifecycle remains backend-specific.

## 2.5 Official Android CLI is the primary AVD lifecycle interface

Google's current documentation explicitly documents:

```text
android emulator create
android emulator list
android emulator start
android emulator stop
```

and marks the old `emulator` management path as deprecated for lifecycle management.

Therefore:

- use `android emulator ...` for normal create/list/start/stop operations;
- detect exact installed capabilities at runtime;
- do not couple Hypr-Phone to Android Studio;
- do not replace or shim the SDK emulator binary;
- permit direct legacy `emulator` execution only in a narrowly isolated compatibility adapter when an advanced launch flag has no official Android CLI equivalent;
- make that compatibility path observable in `doctor`.

## 2.6 Default development AVD for this machine

For an Intel x86_64 Linux machine with KVM, default to an **x86_64 Google APIs image**, not ARM64 emulation.

Default profile philosophy:

- Google APIs image unless Play Store behavior is specifically being tested;
- standard 4 KB page-size image for normal development;
- do not make 16 KB page-size images the daily default;
- 1536–2048 MiB RAM as the initial measurement range;
- hardware virtualization required;
- hardware rendering when the host probe succeeds;
- animation/sync/background reductions only in the reversible `dev` profile;
- Play image and 16 KB image remain explicit testing targets.

Never claim one profile is universally safe for all applications. The tool reports what it changed.

## 2.7 Do not depend on AVD-SLIM

AVD-SLIM is research input, not a runtime dependency.

Borrow the useful ideas:

- lower RAM;
- low-memory mode where valid;
- avoid heavyweight image choices for ordinary development;
- control snapshots;
- reduce background churn;
- reversible service/package changes;
- benchmark before/after.

Do not blindly copy package-disable lists or invasive launch shims.

## 2.8 Storage and RAM are separate products

Never say “the emulator is 1 GB” without specifying the metric.

Hypr-Phone should report separately:

- host RSS / PSS where available;
- configured guest RAM;
- CPU load;
- writable AVD actual disk allocation;
- writable AVD apparent disk size;
- snapshot actual/apparent size;
- shared system-image size;
- SDK component size;
- cache size.

A realistic first target is:

- roughly 1.5–2 GiB running host memory for the lightweight dev AVD where workload allows;
- low idle CPU;
- one controlled golden state instead of uncontrolled snapshot growth;
- fast resume;
- small writable overlay growth.

## 2.9 Destructive operations are always explicit

Never silently:

- wipe an AVD;
- delete a snapshot;
- compact a QCOW image;
- disable Android packages;
- alter a real physical phone's system settings;
- replace SDK binaries;
- delete system images.

Every destructive operation must have:

1. preflight;
2. explicit target;
3. dry-run summary;
4. recovery/backup metadata when applicable;
5. apply;
6. verify;
7. rollback path.

---

# 3. Target architecture

```text
┌──────────────────────────────────────────────────────────────┐
│                        USER SURFACES                         │
│                                                              │
│  hypr-phone CLI   Rofi/Walker   Waybar   optional adapters   │
└───────────────────────────────┬──────────────────────────────┘
                                │ Unix socket / JSON protocol
┌───────────────────────────────▼──────────────────────────────┐
│                         hypr-phoned                          │
│                                                              │
│  canonical state • event bus • target selection • sessions   │
│  ADB events • process watchers • Hyprland events • metrics   │
└───────────────┬────────────────────┬─────────────────┬───────┘
                │                    │                 │
       ┌────────▼────────┐  ┌────────▼────────┐ ┌─────▼──────┐
       │    Physical     │  │       AVD       │ │  Waydroid  │
       │ USB/Wi-Fi/mDNS  │  │ Android Emulator│ │  optional  │
       └────────┬────────┘  └────────┬────────┘ └─────┬──────┘
                └────────────────────┼─────────────────┘
                                     │
                    ┌────────────────▼────────────────┐
                    │ backend capabilities / adapters │
                    │                                 │
                    │ ADB • scrcpy • Android CLI      │
                    │ KDE Connect • Hyprland IPC      │
                    └─────────────────────────────────┘
```

### Rules

- UI never derives truth independently.
- CLI/menu/Waybar consume the same canonical state.
- ADB rows are observations, not identities.
- processes are tracked by PID and start identity, not fuzzy `pgrep` alone;
- windows are tracked by Hyprland address/stable identifier when possible;
- target lifecycle and presentation lifecycle are separate;
- all long-running polling belongs in the daemon, not Waybar;
- every cached state object has `updated_at` and a sequence number.

---

# 4. Canonical data model

The existing `PhoneDevice` model should be migrated rather than expanded forever.

## 4.1 Copy-paste baseline: `src/domain/target.rs`

```rust
use serde::{Deserialize, Serialize};

pub type TargetId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Physical,
    Avd,
    Waydroid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    Usb,
    Wifi,
    Mdns,
    Emulator,
    Container,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetBackend {
    Physical {
        hardware_serial: Option<String>,
        model: Option<String>,
    },
    Avd {
        avd_name: String,
        profile: Option<String>,
        api_level: Option<u32>,
        abi: Option<String>,
    },
    Waydroid {
        session_name: String,
    },
}

impl TargetBackend {
    pub fn kind(&self) -> TargetKind {
        match self {
            Self::Physical { .. } => TargetKind::Physical,
            Self::Avd { .. } => TargetKind::Avd,
            Self::Waydroid { .. } => TargetKind::Waydroid,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdbBinding {
    /// Current handle accepted by `adb -s`. This is NOT stable identity.
    pub serial: String,
    pub transport: TransportKind,
    /// Transient network endpoint when applicable.
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetCapabilities {
    pub adb: bool,
    pub shell: bool,
    pub install_apk: bool,
    pub logcat: bool,
    pub reverse_port: bool,
    pub screenshot: bool,
    pub recording: bool,
    pub clipboard: bool,
    pub scrcpy: bool,
    pub scrcpy_virtual_display: bool,
    pub scrcpy_flex_display: bool,
    pub kde_connect: bool,
    pub lifecycle_start_stop: bool,
    pub lifecycle_wipe: bool,
    pub snapshots: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AndroidTarget {
    /// Stable Hypr-Phone identity. Never derive this from a Wi-Fi endpoint.
    pub id: TargetId,
    pub alias: String,
    pub backend: TargetBackend,
    pub adb: Option<AdbBinding>,
    pub capabilities: TargetCapabilities,
    pub selected: bool,
    pub last_seen_unix_ms: Option<u64>,
}

impl AndroidTarget {
    pub fn kind(&self) -> TargetKind {
        self.backend.kind()
    }

    pub fn adb_serial(&self) -> Option<&str> {
        self.adb.as_ref().map(|v| v.serial.as_str())
    }
}
```

### Stable ID rules

Use deterministic IDs:

```text
physical:<hardware-serial>
avd:<avd-name>
waydroid:<session-name>
```

Fallback for a physical device with no readable hardware serial:

```text
physical:configured:<user-alias>
```

Do **not** use `ip:port` as identity.

---

# 5. Lifecycle state machine

The current boolean-ish connected/mirroring model is too weak.

## 5.1 Copy-paste baseline: `src/domain/lifecycle.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Unavailable,
    Stopped,
    Discovering,
    Connecting,
    Booting,
    Connected,
    LaunchingPresentation,
    WindowReady,
    Visible,
    Hidden,
    Stopping,
    Failed,
}

impl LifecycleState {
    pub fn is_transitional(self) -> bool {
        matches!(
            self,
            Self::Discovering
                | Self::Connecting
                | Self::Booting
                | Self::LaunchingPresentation
                | Self::Stopping
        )
    }

    pub fn is_usable(self) -> bool {
        matches!(
            self,
            Self::Connected | Self::WindowReady | Self::Visible | Self::Hidden
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleError {
    pub code: String,
    pub message: String,
    pub recovery_hint: Option<String>,
    pub occurred_at_unix_ms: u64,
}
```

Required transitions:

```text
Unavailable -> Discovering
Stopped -> Booting -> Connected
Discovering -> Connected
Discovering -> Connecting -> Connected
Connected -> LaunchingPresentation -> WindowReady -> Visible
Visible <-> Hidden
WindowReady -> Visible
Connected -> Stopping -> Stopped
ANY -> Failed
Failed -> Discovering / Connecting / Booting after an explicit retry
```

Do not hide a failed state by flattening it to `Disconnected`.

---

# 6. Runtime sessions

A CLI-local `Vec<ScrcpySession>` cannot work because each command invocation gets a fresh process.

Before the daemon is complete, use an XDG runtime state file as a migration bridge. After the daemon lands, the daemon owns the authoritative state and the file becomes optional crash-recovery metadata.

Runtime directory:

```text
$XDG_RUNTIME_DIR/hypr-phone/
```

Never put active state under `~/.config`.

## 6.1 Session record

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationSession {
    pub session_id: String,
    pub target_id: String,
    pub adb_serial: Option<String>,
    pub pid: u32,
    pub process_start_ticks: Option<u64>,
    pub window_title: String,
    pub window_address: Option<String>,
    pub workspace: String,
    pub profile: String,
    pub app_package: Option<String>,
    pub started_at_unix_ms: u64,
}
```

Why `process_start_ticks` matters: Linux can reuse PIDs. A stale runtime file should not accidentally identify a new unrelated process as a scrcpy session.

On Linux, validate both:

```text
/proc/<pid>/stat exists
process start time matches recorded start time when available
```

---

# 7. `SUPER+P` semantics

This is the most important piece of the product.

The command should be implemented as an explicit resolver, not nested guesses.

## 7.1 Algorithm

```text
resolve selected target
  |
  +-- none -> if exactly one candidate: select it
  |          else target picker
  |
refresh target state
  |
  +-- physical + unreachable
  |      -> try USB/current binding
  |      -> try known mDNS/Wi-Fi reconnect
  |      -> if still unavailable: recovery menu
  |
  +-- AVD + stopped
  |      -> start AVD
  |      -> wait for adb device
  |      -> wait for sys.boot_completed=1
  |
validate presentation session
  |
  +-- recorded PID dead -> remove stale session
  |
resolve Hyprland window by stable session identity
  |
  +-- window exists and visible -> hide special workspace
  +-- window exists and hidden  -> reveal special workspace + focus window
  +-- no window                 -> launch scrcpy -> resolve address -> place -> reveal
```

### Absolute rule

**Never toggle the special workspace merely because Hyprland accepted the dispatcher.**

First prove that the target's presentation window exists.

### Recovery behavior

If scrcpy exists but its window never appears within the configured timeout:

1. terminate only the child process Hypr-Phone launched;
2. clear the stale session;
3. report the captured stderr/log path;
4. retry once if the error looks transient;
5. otherwise leave the target connected and return a clear failure.

Do not infinite-retry.

---

# 8. Hyprland integration

Hyprland is moving quickly. The project must version-gate behavior.

Current Hyprland documentation warns that repeated `hyprctl` calls are synchronous and recommends batching; current releases also expose a Lua-oriented dispatch API.

## 8.1 Version adapter

At startup/doctor:

```text
hyprctl version
```

Parse the semantic version and select an adapter:

```rust
pub enum HyprApi {
    LegacyHyprlang,
    Lua,
}
```

For <=0.54-style installations, keep legacy dispatch syntax.

For >=0.55-style installations, use the current documented Lua dispatch/eval interface.

Never assume a syntax merely because `hyprctl` exists.

## 8.2 Window identity

After launching scrcpy:

- record child PID;
- subscribe/query Hyprland clients;
- match exact PID first;
- store the resulting window address/stable identifier;
- operate on that window explicitly thereafter.

Do not use “currently active window” for placement.

## 8.3 Event consumption

`hypr-phoned` should subscribe to Hyprland events instead of polling clients continuously.

Useful event categories include:

- window open;
- window close;
- workspace change;
- active window change;
- monitor/special-workspace visibility changes where exposed.

The event handler updates cached session state only. Heavy operations should be queued outside the event-reading loop.

---

# 9. Contextual menu contract

The menu should be a state projection, not a hardcoded static list.

## Disconnected physical target

```text
Connect Pixel 8
Pair new Android device…
Use emulator instead…
Target picker…
Doctor
```

## AVD stopped

```text
Start Pixel Dev
Start Pixel Dev (cold)
Start ephemeral test session
Storage / snapshots…
Change target…
Doctor
```

## Target connected, no mirror

```text
Show Android
Open app…
Install APK…
Send…
Screenshot
Developer tools…
Change target…
```

## Visible

```text
Hide Android
Open app…
Screenshot
Record
Device controls…
Developer tools…
Stop mirror
Change target…
```

## Failed

```text
Retry
Show error details
Run doctor for this target
Reset presentation session
Change target…
```

No menu entry may print a stub.

If an action is not implemented, omit it from the menu.

---

# 10. AVD subsystem

Create a dedicated service tree:

```text
src/services/avd/
  mod.rs
  android_cli.rs
  discovery.rs
  lifecycle.rs
  boot.rs
  config_ini.rs
  profile.rs
  metrics.rs
  snapshot.rs
  storage.rs
  mutation.rs
```

## 10.1 CLI surface

```text
hypr-phone avd list
hypr-phone avd create [name]
hypr-phone avd inspect <name>
hypr-phone avd start [name]
hypr-phone avd start [name] --cold
hypr-phone avd start [name] --ephemeral
hypr-phone avd stop [name|serial]
hypr-phone avd restart [name]
hypr-phone avd wipe <name>
hypr-phone avd profile <name> <safe|dev|aggressive|stock>
hypr-phone avd storage [name]
hypr-phone avd snapshot list [name]
hypr-phone avd snapshot create <name> <snapshot>
hypr-phone avd snapshot restore <name> <snapshot>
```

Do not add `delete`, `wipe`, `compact`, or snapshot destruction to the casual contextual menu.

---

# 11. Lightweight AVD profiles

The optimization feature must be called something neutral like **AVD Profiles**, not “magic 1 GB mode”.

## `stock`

No Hypr-Phone mutations.

Purpose:

- reproduce normal emulator behavior;
- validate bugs against a baseline;
- benchmark optimizations honestly.

## `safe` — default lightweight profile

Allowed changes:

- RAM target;
- heap target where supported;
- hardware renderer preference;
- disable emulated camera hardware unless requested;
- disable emulated audio input/output when not needed;
- snapshot policy;
- device frame UI choice;
- boot strategy;
- host-side process priority policy if eventually added.

Do not disable Android apps/services in `safe`.

Initial target:

```text
RAM: 1536 MiB
fallback: 2048 MiB when project/runtime proves 1536 too constrained
```

The profile manager should automatically raise a warning if Android's low-memory killer makes the target unstable.

## `dev`

Everything in `safe`, plus reversible guest changes:

- animation scales can be reduced/disabled;
- automatic sync can be disabled;
- cached/background process limits can be reduced conservatively;
- location/Bluetooth can be disabled only when the project profile says they are not needed;
- optional consumer packages may be suspended only from an allowlisted, version-tested set.

The profile must preserve common developer flows by default:

- ADB;
- WebView;
- network access;
- localhost/reverse ports;
- Firebase/Auth/FCM when Google APIs image supports them;
- app install/debug;
- Expo/React Native Metro connectivity.

## `aggressive`

Experimental and opt-in.

It may include:

- more aggressive package/service suspension;
- low-memory emulator launch flags through a compatibility adapter if the official CLI cannot represent them;
- stricter background limits.

Requirements:

- explicit `--yes` for noninteractive use;
- generated restore manifest;
- exact changed settings printed;
- known-incompatible capabilities listed before apply;
- never the default project profile.

---

# 12. Copy-paste safe AVD config patcher

Do not add a full INI dependency just to modify Android AVD `config.ini`. Preserve unknown lines and comments.

A minimal safe patcher can be isolated and tested.

```rust
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
};

use anyhow::{Context, Result};

#[derive(Debug, Clone, Default)]
pub struct AvdConfigPatch {
    pub values: BTreeMap<String, String>,
}

impl AvdConfigPatch {
    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }

    pub fn apply(&self, path: &Path) -> Result<()> {
        let original = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        let backup = path.with_extension("ini.hypr-phone.bak");
        if !backup.exists() {
            fs::write(&backup, &original)
                .with_context(|| format!("failed to create {}", backup.display()))?;
        }

        let mut seen = BTreeMap::<String, bool>::new();
        for key in self.values.keys() {
            seen.insert(key.clone(), false);
        }

        let mut output = String::with_capacity(original.len() + 256);

        for line in original.lines() {
            let trimmed = line.trim();
            let replacement = trimmed
                .split_once('=')
                .map(|(k, _)| k.trim())
                .and_then(|key| self.values.get(key).map(|value| (key, value)));

            if let Some((key, value)) = replacement {
                output.push_str(key);
                output.push_str(" = ");
                output.push_str(value);
                output.push('\n');
                seen.insert(key.to_string(), true);
            } else {
                output.push_str(line);
                output.push('\n');
            }
        }

        for (key, was_seen) in seen {
            if !was_seen {
                if let Some(value) = self.values.get(&key) {
                    output.push_str(&key);
                    output.push_str(" = ");
                    output.push_str(value);
                    output.push('\n');
                }
            }
        }

        let tmp = path.with_extension("ini.hypr-phone.tmp");
        fs::write(&tmp, output)
            .with_context(|| format!("failed to write {}", tmp.display()))?;
        fs::rename(&tmp, path)
            .with_context(|| format!("failed to replace {}", path.display()))?;

        Ok(())
    }
}
```

Profile code then constructs only keys that have been verified against the installed emulator/AVD format.

Do **not** hardcode unverified config keys just because a blog post mentions them.

---

# 13. AVD boot sequence

`start` is not done when a process appears.

The boot sequence is:

```text
1. resolve target AVD
2. validate SDK/Android CLI
3. validate KVM acceleration
4. validate image exists
5. apply selected non-destructive profile config
6. launch through Android CLI
7. discover emulator serial
8. wait for `adb -s <serial> get-state` == device
9. wait for `getprop sys.boot_completed` == 1
10. verify package manager responds
11. apply reversible guest `dev` profile mutations
12. mark target Connected
13. optionally launch project/app presentation
```

Timeouts must be bounded and configurable.

Suggested defaults:

```text
ADB appearance:      30s
boot completed:      90s
package manager:     20s
scrcpy window:        5s
```

Record per-stage timing for diagnostics.

---

# 14. Snapshot policy

Avoid uncontrolled Quick Boot growth.

Support three policies:

## `normal`

Let the emulator use its normal Quick Boot behavior.

## `golden`

User intentionally creates a known-good development state.

Typical contents:

- boot complete;
- development app installed optionally;
- test account logged in optionally;
- required permissions configured;
- no random previous debug session state.

Normal sessions launched from golden state should avoid unintentionally overwriting it.

## `ephemeral`

Boot from a known base and discard normal session mutations.

Use cases:

- agentic test runs;
- clean repros;
- integration tests;
- repeated Expo/React Native install checks;
- parallel test devices later.

Current emulator release notes document read-only/copy-on-write and file-backed guest RAM behavior. Design around supported emulator behavior instead of inventing a second snapshot system.

---

# 15. Storage engine

`hypr-phone avd storage` should answer **where the disk went**.

Example output:

```text
Pixel_Dev
  writable AVD actual      2.1 GiB
  writable apparent        9.7 GiB
  userdata actual          1.2 GiB
  snapshots actual         612 MiB
  cache actual             85 MiB
  config/metadata          2 MiB

Shared SDK
  system image API 36      2.4 GiB
  emulator                 1.1 GiB
  platform-tools           18 MiB

Reclaimable now            420 MiB
Potential compact saving   ~3.8 GiB (AVD must be stopped)
```

The tool must distinguish:

- actual allocated blocks;
- sparse apparent size;
- shared components that deleting one AVD will **not** reclaim;
- writable per-device storage.

## Safe cleanup

Allowed automatically after confirmation:

- stale lock files proven to have no owner;
- abandoned Hypr-Phone temp files;
- known obsolete Hypr-Phone backups after retention policy;
- user-selected stale snapshots through supported tooling.

## Compaction

`compact` is advanced.

Requirements:

- AVD stopped;
- no emulator process owns the image;
- image format inspected;
- free host disk checked;
- backup/recovery metadata created;
- qemu tooling version/capability detected;
- output image validated before atomic replacement;
- explicit user confirmation.

Do not ship compaction until fixture-based integration tests exist.

## Btrfs-specific optimization

Official emulator release notes warn about snapshot slowdowns on Btrfs because of copy-on-write and recommend `chattr +C` on an **empty** AVD directory before creating new images.

Hypr-Phone may detect this condition and recommend it for newly created AVD storage.

Never automatically run `chattr +C` on a populated existing AVD directory.

---

# 16. Developer mode

This is what turns Hypr-Phone from a phone utility into something worth running every day.

## 16.1 Command

```text
hypr-phone dev
```

Workflow:

```text
identify project
  -> read .hypr-phone.toml
  -> resolve preferred target
  -> start/reconnect target
  -> wait until ready
  -> configure required adb reverse ports
  -> build/install
  -> launch package
  -> optionally open Android window
  -> start filtered logs
```

## 16.2 Project detection order

Detect, don't guess:

- Expo / React Native;
- Gradle Android app;
- Flutter;
- generic Android project.

Project-specific drivers belong under:

```text
src/services/dev/
  mod.rs
  project.rs
  expo.rs
  react_native.rs
  gradle.rs
  flutter.rs
```

Do not mix project build logic into the AVD service.

## 16.3 `.hypr-phone.toml`

Example tailored to an Expo/React Native project:

```toml
version = 1

default_target = "avd:Pixel_Dev"

[android]
package = "com.noxorigin.app"
launch_after_install = true
open_window = true

[dev]
kind = "expo"
reverse_ports = [8081]
logs = true

[avd]
profile = "dev"
boot = "golden"

[scrcpy]
profile = "app"
flex_display = true
```

Per-project config overrides global defaults but must not silently alter destructive policy.

---

# 17. Android app windows

This is one of Hypr-Phone's strongest differentiators.

Current scrcpy supports:

- listing apps;
- starting an app;
- virtual displays;
- flex display;
- forcing an app stop before launch;
- preserving virtual-display content on close where supported.

Use that directly.

## `SUPER+A`

Desired flow:

```text
SUPER+A
  -> instant cached app picker
  -> type "whats"
  -> WhatsApp
  -> dedicated Android window appears
```

The picker should show:

```text
WhatsApp                  com.whatsapp
Chrome                    com.android.chrome
NoxOrigin                 com.noxorigin.app
Settings                  com.android.settings
```

Not just package IDs.

Cache:

- app label;
- package;
- launch capability;
- icon path/cache later;
- last launched timestamp;
- favourite flag.

Refresh asynchronously when target package set changes.

## Per-app rules

```toml
[apps."com.whatsapp"]
width = 440
height = 920
workspace = "special:phone"
audio = true
flex_display = true

[apps."com.noxorigin.app"]
width = 430
height = 900
workspace = "special:dev-phone"
audio = false
flex_display = true
force_stop_before_launch = true
```

Do not force every Android app into one global geometry.

---

# 18. Daemon protocol

Keep the protocol intentionally boring: one JSON object per line over a local Unix socket.

Socket:

```text
$XDG_RUNTIME_DIR/hypr-phone/hypr-phoned.sock
```

Permissions: current user only.

## 18.1 Copy-paste baseline: `src/protocol.rs`

```rust
use serde::{Deserialize, Serialize};

use crate::domain::{lifecycle::LifecycleState, target::AndroidTarget};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Ping,
    GetState,
    Toggle,
    ListTargets,
    SelectTarget { target_id: String },
    Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub protocol_version: u32,
    pub sequence: u64,
    pub updated_at_unix_ms: u64,
    pub selected_target_id: Option<String>,
    pub selected_target_state: LifecycleState,
    pub targets: Vec<AndroidTarget>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Pong { protocol_version: u32 },
    State { state: StateSnapshot },
    Ack,
    Error {
        code: String,
        message: String,
        recovery_hint: Option<String>,
    },
}
```

Protocol rules:

- CLI and daemon refuse incompatible major protocol versions;
- every state mutation increments `sequence`;
- UI may reject an older sequence arriving after a newer one;
- `GetState` is read-only and fast;
- no command should block the socket task while waiting 90 seconds for an emulator boot;
- long actions run as tasks and publish state changes.

---

# 19. Daemon service unit

Install a **user** service, never a root/system daemon.

Copy-paste baseline:

```ini
[Unit]
Description=Hypr-Phone Android control plane
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=%h/.local/bin/hypr-phoned
Restart=on-failure
RestartSec=1
Environment=RUST_BACKTRACE=1

[Install]
WantedBy=graphical-session.target
```

Install location:

```text
~/.config/systemd/user/hypr-phoned.service
```

Setup should print and optionally install this unit only with explicit `--apply`.

---

# 20. Dependency direction

The current dependency set is intentionally small. Add dependencies only when they replace real complexity.

Recommended daemon-phase delta:

```toml
[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
dirs = "5"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
which = "6"
thiserror = "2"
tokio = { version = "1", features = [
  "rt-multi-thread",
  "macros",
  "net",
  "process",
  "signal",
  "time",
  "io-util",
  "sync",
  "fs"
] }
uuid = { version = "1", features = ["v4", "serde"] }
```

Do not add a database. Canonical persistent configuration remains TOML; live state remains in memory with minimal crash-recovery files.

---

# 21. New module tree

Target tree after P3:

```text
src/
  bin/
    hypr-phone.rs
    hypr-phoned.rs

  cli.rs
  protocol.rs

  domain/
    mod.rs
    target.rs
    lifecycle.rs
    session.rs
    status.rs
    capability.rs

  config/
    mod.rs
    schema.rs
    project.rs

  daemon/
    mod.rs
    state.rs
    server.rs
    reducer.rs
    tasks.rs
    events.rs

  services/
    adb/
      mod.rs
      discovery.rs
      ops.rs
      tracker.rs

    avd/
      mod.rs
      android_cli.rs
      discovery.rs
      lifecycle.rs
      boot.rs
      config_ini.rs
      profile.rs
      metrics.rs
      snapshot.rs
      storage.rs
      mutation.rs

    scrcpy/
      mod.rs
      capability.rs
      args.rs
      process.rs
      apps.rs

    hyprland/
      mod.rs
      version.rs
      legacy.rs
      lua.rs
      clients.rs
      events.rs

    dev/
      mod.rs
      project.rs
      expo.rs
      react_native.rs
      gradle.rs
      flutter.rs

    kdeconnect.rs
    clipboard.rs
    notifications.rs

  commands/
    menu.rs
    toggle.rs
    target.rs
    avd.rs
    app.rs
    dev.rs
    status.rs
    doctor.rs
    send.rs
    screenshot.rs
    record.rs
    control.rs
    clipboard.rs
    config.rs
    setup.rs

  ui/
    rofi.rs
    waybar.rs
    format.rs
```

Do not perform this entire file move before P0 behavior is fixed. Refactor only as each phase needs it.

---

# 22. CLI contract

Final intended surface:

```text
hypr-phone
hypr-phone toggle
hypr-phone status [--json|--waybar]
hypr-phone menu

hypr-phone target list
hypr-phone target use <target>
hypr-phone target inspect [target]

hypr-phone device pair ...
hypr-phone device connect ...
hypr-phone device disconnect ...

hypr-phone avd list
hypr-phone avd create ...
hypr-phone avd inspect ...
hypr-phone avd start ...
hypr-phone avd stop ...
hypr-phone avd restart ...
hypr-phone avd profile ...
hypr-phone avd storage ...
hypr-phone avd snapshot ...

hypr-phone app [label-or-package]
hypr-phone dev
hypr-phone install <apk>
hypr-phone logs [package]
hypr-phone clear-data [package]
hypr-phone reverse <host-port> [device-port]
hypr-phone deeplink <url>
hypr-phone intent ...

hypr-phone send <file|url|text>
hypr-phone screenshot
hypr-phone record start|stop|status
hypr-phone control ...
hypr-phone clipboard send|receive

hypr-phone doctor [--json]
hypr-phone setup [--apply]
hypr-phone config ...
```

Unknown subcommands must return Clap usage/error with non-zero status.

Delete the external-subcommand fallback that silently opens the menu.

---

# 23. Waybar contract

Waybar does not run heavyweight discovery.

Normal mode:

```text
hypr-phone status --waybar
```

should perform approximately:

```text
connect local socket -> GetState -> print JSON -> exit
```

Expected latency after warm daemon start: effectively imperceptible; measure it and keep a regression budget.

Examples:

```text
 Pixel 8 · Wi-Fi · 78%
 Pixel Dev · booting 54%
 Pixel Dev · 1.7 GiB
 Android unavailable
```

Tooltip can include:

- target kind;
- ADB transport;
- emulator profile;
- boot/session state;
- battery;
- current app window count;
- RAM/CPU for AVD;
- last warning.

If the daemon is down, Waybar may use one bounded fallback probe and add class `degraded`.

---

# 24. Doctor becomes a serious diagnostic tool

`hypr-phone doctor` should test capability, not binary presence.

Checks:

## Host

- Linux kernel;
- `XDG_RUNTIME_DIR`;
- current Hyprland instance;
- Hyprland version and API adapter;
- KVM existence/permission;
- CPU architecture;
- GPU/renderer basics;
- filesystem type of AVD storage;
- free RAM/disk.

## Android

- `adb` path/version;
- ADB server health;
- Android CLI presence/version;
- emulator component;
- installed system images;
- AVD list;
- platform tools;
- `qemu-img` capability when storage tools requested.

## scrcpy

Probe `--help` once and cache:

- new display;
- flex display;
- start app;
- list apps;
- audio;
- recording;
- V4L2 if later used.

## Integrations

- rofi/wofi/Walker;
- Waybar;
- KDE Connect;
- notification backend;
- systemd user service;
- daemon socket.

## Project

When run in a project:

- project type;
- package ID;
- reverse ports;
- selected target;
- target API/ABI;
- whether the selected lightweight profile conflicts with required capabilities.

Every failure gets:

```text
status
reason
observed value
expected condition
exact next action
```

No generic “something went wrong”.

---

# 25. Implementation phases

The implementation sequence is strict because feature breadth is currently masking weak fundamentals.

## P0 — Make today's Hypr-Phone real

### Scope

1. inspect branch/HEAD/status;
2. fix stable physical identity;
3. remove all menu stubs;
4. remove unknown-command → menu fallback;
5. add persistent runtime session bridge;
6. validate PIDs;
7. resolve/store Hyprland window identity;
8. rewrite `toggle` as state-driven behavior;
9. derive real mirror state;
10. reduce duplicate capability probes;
11. adapt Hyprland syntax by version;
12. update tests;
13. update `.ai/state.md` only from verified results.

### P0 acceptance gate

All must pass:

- no `(stub)` in user-facing paths;
- `SUPER+P` with connected phone/no session launches scrcpy;
- `SUPER+P` with visible mirror hides it;
- `SUPER+P` with hidden mirror reveals it;
- `SUPER+P` with dead recorded PID relaunches;
- `SUPER+P` never reports success for an empty phone workspace;
- menu actions invoke real behavior or are absent;
- bad CLI subcommand returns non-zero usage error;
- `status` truthfully reports mirror process/window state;
- Wi-Fi endpoint change does not create a new physical identity when hardware identity is known;
- existing CLI compatibility tests still pass or have an explicit migration reason.

**Do not start P1 until this is demonstrated.**

---

## P1 — Event-driven `hypr-phoned`

### Build

- second binary;
- local Unix socket;
- protocol v1;
- state reducer;
- ADB tracker;
- Hyprland events;
- scrcpy child watcher;
- daemon health;
- CLI fallback behavior;
- user systemd service.

### Acceptance

- Waybar status does not launch ADB or scrcpy help probes;
- disconnect/reconnect updates state without manual polling;
- closing scrcpy updates state immediately;
- opening/closing matching Hyprland window updates visibility state;
- daemon restart reconstructs enough state to avoid lying;
- idle daemon memory and CPU are measured and documented.

---

## P2 — `AndroidTarget`

### Build

- migrate physical model;
- target registry;
- target selection;
- target-aware operations;
- `target list/use/inspect`;
- compatibility mapping for old device commands.

### Acceptance

The same high-level functions work against physical and synthetic AVD test doubles without backend-specific command duplication:

- shell;
- install;
- screenshot;
- logs;
- reverse;
- app launch.

---

## P3 — Native AVD manager

### Build

- Android CLI capability adapter;
- AVD discovery;
- create/list/start/stop;
- boot readiness;
- selected AVD target;
- metadata inspection;
- KVM preflight;
- project target preference.

### Acceptance

From a shell with no Android Studio UI:

```text
hypr-phone avd create Pixel_Dev
hypr-phone target use avd:Pixel_Dev
hypr-phone toggle
```

must result in a booted, ready Android target and visible usable Android window.

---

## P4 — Lightweight profiles + metrics + storage

### Build

- `stock`, `safe`, `dev`, `aggressive`;
- config patch transaction;
- guest setting transaction;
- restore manifest;
- runtime memory/CPU metrics;
- disk breakdown;
- snapshot policy;
- golden/ephemeral flow;
- optional advanced compaction later.

### Acceptance

For the same AVD/image/project:

benchmark and save:

```text
stock cold boot
stock warm boot
stock idle RAM/CPU
safe cold/warm/idle
safe disk
project launch success
Firebase/Auth/FCM-required smoke checks when the project needs them
```

Only publish improvement percentages from captured data.

---

## P5 — Developer workflow

### Build

- `.hypr-phone.toml`;
- project detection;
- Expo/React Native first;
- Gradle next;
- Flutter later;
- reverse ports;
- build/install/launch;
- package-filtered logs.

### Acceptance

From the NoxOrigin app repository, one command should be able to reach:

```text
target ready -> Metro/reverse ready -> app installed -> app launched -> useful logs
```

without Android Studio.

---

## P6 — Desktop-quality app windows

### Build

- cached labels;
- favourites/recents;
- per-app profiles;
- virtual display/flex display;
- deterministic window placement;
- fast `SUPER+A`.

### Acceptance

Launching a commonly used Android app should feel comparable to launching a normal desktop app from Rofi.

---

## P7 — Optional integrations

Only now add:

- Waydroid backend;
- richer KDE Connect actions;
- notification history/actions;
- RemoteDesktop/phone-as-touchpad integration;
- V4L2 camera forwarding;
- file drop targets;
- presentation controls;
- SMS where supported.

Do not block the core product on these.

---

# 26. Testing strategy

## Unit tests

Required areas:

- stable physical target identity;
- target selection;
- lifecycle transitions;
- protocol serialization;
- config migration;
- AVD config patch preservation;
- mutation rollback;
- app metadata parsing;
- status projection;
- menu projection.

## Fake-binary integration tests

Create temporary fake executables for:

```text
adb
android
scrcpy
hyprctl
kdeconnect-cli
rofi
notify-send
```

Tests control behavior through environment variables and fixture files.

Required scenarios:

- ADB timeout;
- unauthorized device;
- changed Wi-Fi endpoint;
- scrcpy exits before window creation;
- scrcpy PID stale/reused;
- Hyprland old/new syntax adapter;
- AVD slow boot;
- AVD boot failure;
- target disappears mid-command;
- malformed Android CLI output;
- daemon stale state/resync;
- rollback after partial profile mutation.

## Real runtime gates

Static tests are not enough.

Label evidence explicitly:

```text
STATIC
HOST RUNTIME
PHYSICAL DEVICE
AVD
PROJECT E2E
```

Do not write “verified” when only `cargo test` ran.

---

# 27. Performance budgets

Track regressions from the beginning.

Initial budgets:

| Path | Budget |
|---|---:|
| `hypr-phone status --waybar` with daemon | < 50 ms typical |
| contextual menu state fetch | < 100 ms typical |
| physical hidden-window reveal | < 300 ms perceived |
| existing AVD hidden-window reveal | < 300 ms perceived |
| daemon idle CPU | effectively ~0 when no events |
| daemon RSS | measure; keep intentionally small |
| app picker open from cached metadata | < 150 ms typical |

AVD boot budgets depend on machine/image and must be measured rather than invented.

---

# 28. Logging and diagnostics

Use structured logs internally.

Per session log directory:

```text
$XDG_STATE_HOME/hypr-phone/logs/
```

or fallback:

```text
~/.local/state/hypr-phone/logs/
```

Capture:

- command;
- target ID;
- child PID;
- child stderr/stdout tail where useful;
- transition durations;
- failure code;
- Hyprland resolution result;
- AVD boot stage timings.

Default CLI output remains concise.

`--verbose` prints more.

`doctor` points to exact log path.

---

# 29. Configuration hierarchy

Order from highest priority to lowest:

```text
CLI flag
project .hypr-phone.toml
user ~/.config/hypr-phone/config.toml
built-in defaults
```

Do not mutate the global config just because a project requests a different AVD profile.

Suggested schema direction:

```toml
config_version = 3
selected_target = "physical:<serial>"

[daemon]
enabled = true

[ui]
launcher = "auto"

[hyprland]
workspace = "special:phone"

[avd.defaults]
profile = "safe"
ram_mb = 1536
snapshot_policy = "golden"

[physical]
auto_reconnect = true

[scrcpy.defaults]
max_fps = 60
audio = false
```

Migrations must be deterministic and tested.

---

# 30. Security boundaries

- no network daemon;
- Unix socket current-user only;
- no telemetry;
- no cloud account;
- no root daemon;
- no `sh -c` for user-controlled values when argv APIs work;
- validate package names;
- validate target IDs;
- canonicalize file paths before destructive storage operations;
- refuse AVD mutation if target directory escapes known Android AVD roots;
- never run arbitrary project commands from config without an explicit opt-in design;
- never expose pairing codes in logs longer than necessary.

---

# 31. What not to build

Do not build these before the core is excellent:

- Electron GUI;
- Tauri GUI;
- custom Android emulator;
- custom scrcpy protocol;
- custom ADB implementation;
- cloud device sync;
- mobile companion app;
- Noctalia-only core;
- Quickshell-only core;
- custom notification daemon;
- database;
- cross-platform abstraction layer;
- giant plugin framework.

Adapters can come later because the daemon protocol gives them a clean boundary.

---

# 32. Research facts that affect implementation

Verified during the 2026-09-19 research pass:

1. Current Android documentation exposes the official Android CLI AVD lifecycle commands `android emulator create/list/start/stop` and describes the older emulator management route as deprecated for that role.
2. Android Emulator 37.1.11 stable release notes document snapshot-path commands, read-only/copy-on-write behavior, file-backed guest RAM behavior, and Btrfs CoW guidance.
3. AVD-SLIM currently reports substantial RAM reductions by combining lower RAM, image choice, renderer control, guest reductions, and golden snapshots; community results are not uniform, so Hypr-Phone must benchmark and keep mutations reversible.
4. Current scrcpy documentation supports app listing, `--start-app`, `--new-display`, `--flex-display`, recording and related Android-window workflows.
5. Hyprland documentation warns against many synchronous `hyprctl` calls, recommends batching where relevant, and current releases expose Lua-oriented dispatch APIs while old versions use legacy syntax.

References:

- https://developer.android.com/studio/run/emulator-commandline
- https://developer.android.com/tools/agents/android-cli
- https://developer.android.com/studio/releases/emulator
- https://developer.android.com/studio/run/managing-avds
- https://github.com/kdbhalala/avdslim
- https://www.reddit.com/r/androiddev/comments/1whu9a8/tired_of_android_emulator_eating_8_gb_of_ram/
- https://github.com/Genymobile/scrcpy/blob/master/doc/device.md
- https://github.com/Genymobile/scrcpy/blob/master/doc/virtual-display.md
- https://wiki.hypr.land/Configuring/Advanced-and-Cool/Using-hyprctl/
- https://wiki.hypr.land/0.54.0/IPC/

---

# 33. Immediate implementation brief for the next coding agent

Copy this entire section as the first implementation prompt after placing this plan in the repository.

```text
Work on the CURRENT checked-out branch of namikofficial/hypr-phone.

Treat HYPR_PHONE_MASTER_IMPLEMENTATION_PLAN_FINAL.md as product direction, but trust current source and runtime over historical state files.

Do ONLY P0 in this session. Do not implement the daemon, AVD manager, Waydroid, broad KDE Connect expansion, or speculative UI work yet.

Before editing:
1. print git status, branch and HEAD;
2. inspect current menu.rs, toggle.rs, status.rs, device identity code, Hyprland service, scrcpy service, CLI parser and tests;
3. preserve unrelated dirty work;
4. run the current test/build baseline and record failures that predate your edits.

P0 goals:
- remove every user-facing menu stub;
- remove external/unknown-subcommand fallback to the menu;
- repair physical device identity so changing Wi-Fi endpoints does not redefine a known device when stable hardware identity exists;
- add a small XDG_RUNTIME_DIR session bridge for scrcpy until hypr-phoned exists;
- track launched scrcpy PID and validate stale PID state;
- resolve/store the corresponding Hyprland window address or exact stable selector;
- make toggle inspect target + process + window state before deciding show/hide/launch;
- never report success just because togglespecialworkspace returned OK;
- derive real mirror state in status;
- probe scrcpy capabilities once per command, not repeatedly;
- detect Hyprland version and keep compatibility with the installed syntax instead of assuming one dispatcher API;
- add regression tests for every state transition above.

Menu behavior:
- actions that need an argument must open a small submenu/input flow or be omitted until a real flow exists;
- no '(stub)' output is allowed;
- direct actions must call the same underlying command/service functions used by CLI subcommands;
- do not fork a second implementation inside menu.rs.

Toggle acceptance matrix:
A. connected target + no session/window -> launch and show;
B. connected target + valid hidden window -> show/focus;
C. connected target + valid visible window -> hide;
D. stale session PID -> clean stale state and relaunch;
E. process exists but window creation fails -> bounded retry, then useful error;
F. disconnected wireless target + known endpoint -> bounded reconnect then continue;
G. no target -> useful target/setup diagnostic;
H. unauthorized target -> explicit authorization error;
I. empty special workspace -> never treated as successful Android presentation.

Testing:
- keep cargo fmt clean;
- cargo clippy --all-targets --all-features -- -D warnings;
- cargo test --all;
- use fake adb/scrcpy/hyprctl binaries for integration tests;
- if a real connected target is available, run a real smoke test but label it separately from static verification.

Do not update .ai/state.md or README to claim completion until the acceptance matrix is demonstrated. At the end, update them only with facts actually verified in this run.
```

---

# 34. Final implementation order

Do not reorder this without evidence:

```text
P0  make current product truthful and reliable
 ↓
P1  hypr-phoned + event state
 ↓
P2  AndroidTarget abstraction
 ↓
P3  official AVD lifecycle
 ↓
P4  lightweight profiles + metrics + storage
 ↓
P5  project-aware developer workflow
 ↓
P6  desktop-quality Android app windows
 ↓
P7  optional Waydroid/KDE/remote-input extras
```

The most important constraint is unchanged:

> **Do not keep adding features to a tool whose primary shortcut is not yet trustworthy.**

When P0 is complete, Hypr-Phone should already be useful enough to keep running. When P3–P5 are complete, it should replace most of the reasons to open Android Studio during normal Expo/React Native development. When P6 is complete, Android apps should feel like intentional members of the Hyprland desktop instead of remote phone pixels.

That is the product.

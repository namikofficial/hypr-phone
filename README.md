# hypr-phone

**Press Super+P. Your phone appears.**

hypr-phone is a Hyprland-native Android presence layer. It discovers your
Android device (USB or wireless), launches scrcpy into a dedicated special
workspace, exposes contextual actions through rofi/wofi, and gives Waybar a
stable status interface — all from a single CLI.

It does not replace scrcpy, KDE Connect, ADB, or Hyprland. It is the
orchestration layer that makes them feel like one tool.

---

## 60-second setup

```bash
# Arch dependencies
sudo pacman -S android-tools scrcpy wl-clipboard libnotify rofi

# Optional
sudo pacman -S kdeconnect wofi

# Build
cargo install --path .
# or use ./scripts/install-global.sh

# First run
hypr-phone setup            # prints suggested Hyprland bindings + Waybar snippet
hypr-phone config init      # write default config
hypr-phone doctor           # verify environment
```

Add these to your `hyprland.conf`:

```ini
bind = SUPER, P, exec, hypr-phone toggle
bind = SUPER SHIFT, P, exec, hypr-phone menu
bind = SUPER, A, exec, hypr-phone app
bind = SUPER SHIFT, S, exec, hypr-phone screenshot
```

Pair a phone once:

```yaml
# Wireless debugging on Android (Developer options) shows
#   IP address & Port  → use for pair
#   Pairing code       → shown after tapping "Pair device with pairing code"
hypr-phone device pair 192.168.1.20:37123 123456
hypr-phone device connect 192.168.1.20:5555
```

From then on:

```bash
hypr-phone toggle   # SUPER+P — phone appears
```

---

## Daily workflow

```bash
hypr-phone                  # contextual launcher (auto-detects state)
hypr-phone toggle           # show/hide phone workspace + (re)launch mirror
hypr-phone app              # open any Android app as a Hyprland window
hypr-phone app whatsapp     # launch a specific package
hypr-phone screenshot       # capture to ~/Pictures/hypr-phone/, notify
hypr-phone record start     # record screen; stop later
hypr-phone send ~/Downloads/file.pdf   # route via KDE Connect or adb push
hypr-phone send https://example.com    # open URL on phone
hypr-phone send "paste me"             # phone clipboard
hypr-phone control home     # one of: home, back, recents, lock, wake,
                            #        screen-off, volume-up, volume-down,
                            #        mute, notifications, quick-settings, rotate
hypr-phone status           # canonical status
hypr-phone status --waybar  # Waybar JSON
hypr-phone doctor           # thorough environment check
hypr-phone doctor --json    # machine-readable diagnostic
```

### Contextual menu

`hypr-phone` (no arguments) opens a rofi/wofi menu whose actions change
based on the current state:

**Phone connected:**

```
󰄜 Toggle phone
󰀻 Open Android app…
󰈔 Send (file / url / text)
󰹑 Screenshot
󰖯 Record screen
󰋊 Device controls…
󰋏 Clipboard sync
󰂃 Battery (KDE Connect)
󰂃 Find / ring phone
󰒓 Doctor / diagnostics
```

**No phone connected:**

```
Connect phone
Pair new phone
Reconnect last phone
Doctor / diagnostics
Setup
```

### Apps as Hyprland windows

`hypr-phone app` lists installed third-party apps, lets you fuzzy-pick one,
and opens it on a virtual display using `scrcpy --new-display --start-app`.
The window gets its own Hyprland title and a sensible placement.

```bash
hypr-phone app                   # picker
hypr-phone app com.whatsapp      # specific package
```

Requires `scrcpy` ≥ 2.0 (built-in `--new-display`/`--start-app`).

---

## Waybar integration

```json
{
    "custom/phone": {
        "exec": "hypr-phone module",
        "format": "{}",
        "on-click": "hypr-phone toggle",
        "on-click-right": "hypr-phone menu",
        "interval": 5
    }
}
```

The module outputs JSON with these classes:

- `connected` — phone reachable
- `disconnected` — no device
- `connecting` — reconnect in progress
- `mirroring` — scrcpy running
- `error` — failure

Tooltip contains device name, transport, battery (if KDE Connect is
paired), and mirror state.

---

## Profiles

scrcpy is exposed as named profiles in `~/.config/hypr-phone/config.toml`:

| Profile        | Use                                       |
|----------------|-------------------------------------------|
| `default`      | Everyday phone control                    |
| `low_latency`  | Responsive screen-on-device               |
| `presentation` | High-quality, show touches                |
| `desk`         | Phone off, mouse + keyboard optimized     |
| `app`          | Virtual display for an Android app        |
| `record`       | Recording defaults                        |

Customize or add new profiles in the config:

```toml
[mirror.profiles.gaming]
video_bit_rate = "12M"
max_fps = 120
audio = false
turn_screen_off = true
args = ["--no-control"]   # any extra raw scrcpy flags
```

Switch via `--profile`:

```bash
hypr-phone toggle --profile desk
hypr-phone app whatsapp --profile app
```

---

## Architecture

```
src/
  domain/      PhoneDevice, ScrcpySession, PhoneStatus, errors
  services/
    adb/       discovery (mDNS, USB, wireless) + ops (push, shell, screenshot, clipboard)
    hyprland/  runtime IPC, no windowrulev2 generation
    scrcpy/    profile resolution, argument construction
    kdeconnect/  battery, ring, share
    clipboard/ wl-copy/wl-paste
    notifications/ notify-send
  commands/    user-facing verbs (toggle, status, app, send, record, …)
  ui/          waybar JSON, rofi/wofi launcher
  config/      TOML v2 schema with v1→v2 migration
  cli.rs       command parser
  main.rs      thin dispatcher (~150 lines)
```

**Key design choices:**

- **Device identity ≠ transport.** A `PhoneDevice` has a stable id and any
  number of `Transport`s (USB, Wi-Fi, mDNS). Endpoints can change without
  breaking device selection.
- **Runtime Hyprland placement.** No static `windowrulev2` rules;
  `hyprctl dispatch` controls the window placement at runtime.
- **Canonical `PhoneStatus`.** One model consumed by CLI output, menu,
  Waybar — no UI re-derives truth.
- **Capability detection.** `doctor` parses scrcpy `--help` to determine
  which features work in your installed version.

---

## Security

- All external commands use argv arrays — no shell injection.
- App package names are validated against `<reverse.dns>` shape.
- No network daemon. The future `hypr-phoned` will use a local-user
  Unix socket only.
- No telemetry. No cloud. No accounts.

---

## Roadmap

- [x] P0: device model, toggle, contextual menu, doctor
- [x] P1: app launcher, intelligent send, recording, Waybar, setup
- [ ] P2: optional `hypr-phoned` for event-driven device tracking
- [ ] P2: shell completion + man page

See [docs/](docs/) for detailed usage, troubleshooting, and profiles.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

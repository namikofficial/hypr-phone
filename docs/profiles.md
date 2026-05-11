# Profiles and Config

hypr-phone uses a TOML config for devices, scrcpy profiles, and Hyprland integration.

Config path:

```txt
~/.config/hypr-phone/config.toml
```

Print configured path:

```bash
hypr-phone config path
```

Create starter config:

```bash
hypr-phone config init
```

## Full v0.1 example

```toml
default_device = "pixel"

[devices.pixel]
name = "Pixel"
serial = "192.168.1.23:5555"

[profiles.default]
max_size = 1080
bit_rate = "8M"
max_fps = 60
audio = true
turn_screen_off = false
stay_awake = true

[profiles.low_latency]
max_size = 720
bit_rate = "4M"
max_fps = 60
audio = false
turn_screen_off = true
stay_awake = true

[profiles.presentation]
max_size = 1080
bit_rate = "12M"
max_fps = 30
audio = true
turn_screen_off = false
stay_awake = true

[hyprland]
enabled = true
special_workspace = "phone"
window_title_prefix = "hypr-phone"
floating = true
center = true
width = 420
height = 900
```

## Device aliases

Each device entry gives a stable name for CLI usage:

```bash
hypr-phone mirror pixel --profile default
```

Without alias, you can still connect via endpoint:

```bash
hypr-phone connect 192.168.1.23:5555
```

## Profile recommendations

### default

- General daily use
- Good quality + responsiveness balance

### low_latency

- Better input responsiveness
- Lower bandwidth + reduced visual fidelity

### presentation

- Higher image quality
- Lower FPS to reduce load in demos/recordings

## Window behavior

`window_title_prefix = "hypr-phone"` pairs with Hyprland rules matching:

```ini
title:^(hypr-phone:.*)$
```

Use this to auto-float and move mirror windows to a special workspace.

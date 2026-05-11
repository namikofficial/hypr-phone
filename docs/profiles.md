# Profiles and Config

hypr-phone uses a TOML config for scrcpy profiles and Hyprland placement.

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
device_serial = "192.168.1.23:5555"
scrcpy_args = []

[mirror]
device_serial = "192.168.1.23:5555"
profile = "default"
window_title_prefix = "hypr-phone"

[mirror.hyprland]
enabled = true
workspace = "special:phone"
width = 420
height = 900
retry_timeout_ms = 3000
retry_interval_ms = 200

[mirror.profiles.default]
max_size = 1080
video_bit_rate = "8M"
max_fps = 60
audio = true
turn_screen_off = false
stay_awake = true

[mirror.profiles.low_latency]
max_size = 720
video_bit_rate = "4M"
max_fps = 60
audio = false
turn_screen_off = true
stay_awake = true

[mirror.profiles.presentation]
max_size = 1080
video_bit_rate = "12M"
max_fps = 30
audio = true
turn_screen_off = false
stay_awake = true
```

## Device selection

Mirror chooses the target in this order:

1. `hypr-phone mirror <serial>`
2. `mirror.device_serial` in config
3. top-level `device_serial` (legacy compatibility)
4. first connected device from `adb devices -l`

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

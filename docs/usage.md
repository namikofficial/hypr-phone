# Usage

hypr-phone v0.1 is a terminal-first Android companion for Hyprland.

## Command list (v0.1)

```bash
hypr-phone doctor
hypr-phone devices
hypr-phone pair <ip:port> <pairing-code>
hypr-phone connect <ip:port>
hypr-phone disconnect <serial>
hypr-phone mirror [serial] [--profile default]
hypr-phone mirror [serial] --profile low_latency
hypr-phone mirror [serial] --profile presentation
hypr-phone module
hypr-phone menu
hypr-phone config path
hypr-phone config init
```

## Typical workflow

### 1) Check environment

```bash
hypr-phone doctor
```

### 2) List available devices

```bash
hypr-phone devices
hypr-phone devices --json
```

### 3) Wireless pairing + connect

```bash
hypr-phone pair 192.168.1.23:37123 123456
hypr-phone connect 192.168.1.23:5555
```

### 4) Mirror using a profile

```bash
hypr-phone mirror 192.168.1.23:5555 --profile low_latency
```

or:

```bash
hypr-phone mirror --profile default
```

If no serial is passed, hypr-phone tries config defaults and then the first connected ADB device.

### 5) Show status module output

```bash
hypr-phone module
```

Connected output example:

```json
{
  "text": "󰄜 Pixel",
  "tooltip": "Android device connected: Pixel\nADB: 192.168.1.23:5555",
  "class": "connected"
}
```

Disconnected output example:

```json
{
  "text": "󰄛 No phone",
  "tooltip": "No Android device connected",
  "class": "disconnected"
}
```

### 6) Open quick menu

```bash
hypr-phone menu
```

Minimum v0.1 menu actions:

- Mirror default device
- Mirror low latency
- List devices
- Pair wireless ADB
- Connect wireless ADB

## Mirror profile behavior

Profile fields map to scrcpy flags:

- `max_size` -> `--max-size`
- `video_bit_rate` -> `--video-bit-rate`
- `max_fps` -> `--max-fps`
- `audio` -> audio enabled/disabled behavior
- `turn_screen_off` -> `--turn-screen-off`
- `stay_awake` -> `--stay-awake`

See [docs/profiles.md](profiles.md) for config examples.

## Hyprland behavior notes

After launching scrcpy, hypr-phone should try to apply Hyprland actions:

```bash
hyprctl dispatch movetoworkspace special:phone
hyprctl dispatch togglefloating
hyprctl dispatch resizeactive exact 420 900
hyprctl dispatch centerwindow
```

Because the window may appear late, retry matching for up to ~3 seconds.
If Hyprland integration fails, mirror should still open.

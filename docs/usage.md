# Usage

hypr-phone is a terminal-first Android companion for Hyprland with v0.2/v0.3 command coverage and v0.4+ scaffolding.

## Command list (current)

```bash
hypr-phone doctor
hypr-phone devices
hypr-phone pair <ip:port> <pairing-code>
hypr-phone connect [ip:port]
hypr-phone reconnect [alias|ip:port]
hypr-phone disconnect <serial>
hypr-phone mirror [serial] [--profile default]
hypr-phone screenshot [alias|serial] [output.png]
hypr-phone push <local> <remote> [alias|serial]
hypr-phone pull <remote> <local> [alias|serial]
hypr-phone install-apk <app.apk> [alias|serial] [--reinstall]
hypr-phone shell [--target alias|serial] <command...>
hypr-phone clipboard send [--target alias|serial] [--text "..."]
hypr-phone clipboard receive [--target alias|serial]
hypr-phone kde devices
hypr-phone kde battery [--target alias|kde-id]
hypr-phone kde ring [--target alias|kde-id]
hypr-phone kde notify [--target alias|kde-id] --title "..." --body "..."
hypr-phone kde media [--target alias|kde-id] <play|pause|play-pause|stop|next|previous>
hypr-phone module
hypr-phone menu
hypr-phone config path
hypr-phone config init
hypr-phone config status
hypr-phone config migrate
hypr-phone gui status
hypr-phone gui generate-rules
```

`gui generate-rules` currently emits `windowrulev2` rules for broad compatibility with existing Hyprland configs.

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

Or use guided mode:

```bash
hypr-phone connect
```

Guided mode asks for pair endpoint + code, runs `adb pair`, then asks for connect endpoint (defaulting to the same IP on port `5555`) and runs `adb connect`.

If `qrencode` is installed, guided mode also prints a terminal QR helper payload during pairing. Without `qrencode`, it falls back to plain text instructions.

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
- Connect wireless ADB (guided flow)

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


## v0.2 helper examples

```bash
hypr-phone reconnect pixel
hypr-phone screenshot pixel ./pixel.png
hypr-phone push ./song.mp3 /sdcard/Music/song.mp3 pixel
hypr-phone pull /sdcard/DCIM/Camera ./camera pixel
hypr-phone install-apk ./app-debug.apk pixel --reinstall
hypr-phone shell --target pixel dumpsys battery
hypr-phone clipboard send --target pixel --text "hello from hyprland"
hypr-phone clipboard receive --target pixel
```

Clipboard send wraps payloads with remote-shell-safe quoting before running `adb shell cmd clipboard set text ...`, so special characters are preserved literally.

Manual device check:

```bash
hypr-phone clipboard send --target pixel --text "demo $(date) ; (ok) `echo x` can't"
hypr-phone clipboard receive --target pixel
```

## v0.3 KDE Connect examples

```bash
hypr-phone kde devices
hypr-phone kde battery --target pixel
hypr-phone kde ring --target pixel
hypr-phone kde notify --target pixel --title "hypr-phone" --body "Sync complete"
hypr-phone kde media --target pixel play
```

If `kdeconnect-cli` is not installed, `hypr-phone kde ...` commands fail with a clear actionable message.

## v0.4/v1 groundwork commands

```bash
hypr-phone config status
hypr-phone config migrate
hypr-phone gui status
hypr-phone gui generate-rules
```

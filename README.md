# hypr-phone

A Wayland-native Android companion for Hyprland.

`hypr-phone` brings together ADB, scrcpy, KDE Connect, wl-clipboard, desktop notifications, and Hyprland window rules into one clean workflow.

Instead of manually running `adb`, pairing wireless devices, launching `scrcpy`, moving windows, writing Waybar modules, and creating rofi scripts, `hypr-phone` gives Hyprland users a single CLI-first Android control center.

It is not a replacement for scrcpy or KDE Connect. It is the missing native orchestration layer for Hyprland users.

## Why?

Linux already has great Android tools, but they are fragmented:

- `adb` for pairing, connect, shell, transfer, debugging
- `scrcpy` for fast screen mirroring and control
- `kdeconnect-cli` for desktop-device integration
- `wl-clipboard` for Wayland clipboard flows
- notification daemons (`notify-send`) for desktop feedback
- `hyprctl` for workspace/window automation

`hypr-phone` is the orchestration layer that unifies these tools for Hyprland users.

## Features

v0.1 is a **CLI-first MVP**:

- List Android devices (`hypr-phone devices`)
- Pair wireless ADB (`hypr-phone pair <ip:port> <pairing-code>`)
- Connect wireless ADB (`hypr-phone connect <ip:port>`)
- Launch `scrcpy` using named profiles
- Hyprland-friendly mirroring workflow (floating + special workspace)
- Waybar/Wayle-ready JSON state output (`hypr-phone module`)
- Quick launcher menu for rofi/wofi (`hypr-phone menu`)

> No GUI in v0.1. The goal is a reliable terminal-first tool.

## Install

### Arch Linux dependencies

```bash
sudo pacman -S android-tools scrcpy wl-clipboard libnotify rofi
```

Optional:

```bash
sudo pacman -S kdeconnect
sudo pacman -S wofi
```

### Build

```bash
git clone https://github.com/<you>/hypr-phone.git
cd hypr-phone
cargo build --release
```

Binary path:

```bash
./target/release/hypr-phone
```

See [docs/install.md](docs/install.md) for full setup.

## Usage

Initialize config and check environment:

```bash
hypr-phone config init
hypr-phone doctor
```

Core v0.1 command set:

```bash
hypr-phone doctor
hypr-phone devices
hypr-phone pair <ip:port> <pairing-code>
hypr-phone connect <ip:port>
hypr-phone disconnect <serial>
hypr-phone mirror [serial] --profile default
hypr-phone mirror [serial] --profile low_latency
hypr-phone mirror [serial] --profile presentation
hypr-phone module
hypr-phone menu
hypr-phone config path
hypr-phone config init
```

See [docs/usage.md](docs/usage.md) for detailed examples.

## Hyprland Integration

Use title-based rules for the scrcpy window launched by hypr-phone:

```ini
windowrulev2 = float, title:^(hypr-phone:.*)$
windowrulev2 = size 420 900, title:^(hypr-phone:.*)$
windowrulev2 = center, title:^(hypr-phone:.*)$
windowrulev2 = workspace special:phone, title:^(hypr-phone:.*)$
```

Ready-to-copy file: [examples/hyprland.conf](examples/hyprland.conf)

## Waybar/Wayle Integration

`hypr-phone module` outputs compact JSON for status modules:

```json
{
  "text": "󰄜 Pixel",
  "tooltip": "Android device connected: Pixel\nADB: 192.168.1.23:5555",
  "class": "connected"
}
```

- Waybar example: [examples/waybar-module.json](examples/waybar-module.json)
- Wayle notes: [examples/wayle-module.md](examples/wayle-module.md)

## Roadmap

### v0.2

- Device aliases
- Better wireless reconnect
- Screenshot command
- File push/pull
- APK install
- ADB shell shortcut
- Clipboard send/receive helpers

### v0.3

- KDE Connect bridge
- Notification sync helpers
- Battery status
- Ring/find phone action
- Media control

### v0.4

- Tauri GUI
- Device cards
- Pairing wizard
- Profile editor
- Hyprland rule generator

### v1.0

- Polished GUI + tray/status
- Stable config format
- AUR + Nix + distro packaging
- Dedicated docs site

## Contributing

Contributions are welcome:

1. Open an issue with your use-case/problem.
2. Keep changes focused and testable.
3. Prefer practical CLI UX improvements.
4. Include docs/examples updates when behavior changes.

## Showcase Blurb

Built `hypr-phone`, an open-source Wayland-native Android companion for Hyprland that unifies ADB device management, wireless pairing, scrcpy screen mirroring, special workspace integration, status module output, and rofi/wofi quick actions into a polished Linux workflow.

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

Current CLI includes v0.1 foundations plus major v0.2/v0.3 helpers:

- Device listing + wireless pair/connect/reconnect (`devices`, `pair`, `connect`, `reconnect`)
- Device aliases in stable config v1 (`[devices.aliases.<name>]`)
- Mirroring profiles + Hyprland placement (`mirror`)
- Screenshot / file transfer / APK install / shell helpers (`screenshot`, `push`, `pull`, `install-apk`, `shell`)
- Clipboard send/receive bridge (`clipboard send`, `clipboard receive`)
- KDE Connect bridge (`kde devices|battery|ring|notify|media`) with graceful fallback if `kdeconnect-cli` is missing
- Waybar/Wayle-ready JSON state output (`module`)
- Quick launcher menu for rofi/wofi (`menu`)
- GUI/tray scaffolding + rule generation (`gui status`, `gui generate-rules`)

> GUI remains scaffold-stage; CLI is still the primary workflow.

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
git clone https://github.com/namikofficial/hypr-phone.git
cd hypr-phone
cargo build --release
```

Install globally (user-local) in one command:

```bash
./scripts/install-global.sh
```

The installer runs `cargo install --path . --force`, then creates/updates:

```txt
~/.local/bin/hypr-phone -> ${CARGO_HOME:-$HOME/.cargo}/bin/hypr-phone
```

If `hypr-phone` is not found in new terminals, ensure your shell PATH includes local bins:

```bash
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
```

See [docs/install.md](docs/install.md) for full setup and first-run checks.

## Usage

Initialize config and check environment:

```bash
hypr-phone config init
hypr-phone doctor
```

Core command set:

```bash
hypr-phone doctor
hypr-phone devices [--json]
hypr-phone pair <ip:port> <pairing-code>
hypr-phone connect [ip:port]
hypr-phone reconnect [alias|ip:port]
hypr-phone disconnect <serial>
hypr-phone mirror [serial] --profile default
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
hypr-phone config path|init|status|migrate
hypr-phone gui status|generate-rules
```

See [docs/usage.md](docs/usage.md) for detailed examples.

Tip: `hypr-phone connect` without an endpoint launches a guided pairing/connect flow. If `qrencode` is installed, it also prints a terminal QR helper payload during pairing.

## Hyprland Integration

Apply the example rules now:

```bash
mkdir -p ~/.config/hypr
cp ./examples/hyprland.conf ~/.config/hypr/hypr-phone.conf
grep -qxF 'source = ~/.config/hypr/hypr-phone.conf' ~/.config/hypr/hyprland.conf || echo 'source = ~/.config/hypr/hypr-phone.conf' >> ~/.config/hypr/hyprland.conf
hyprctl reload
```

Rules file: [examples/hyprland.conf](examples/hyprland.conf)

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

- ✅ Device aliases
- ✅ Better wireless reconnect
- ✅ Screenshot command
- ✅ File push/pull
- ✅ APK install
- ✅ ADB shell shortcut
- ✅ Clipboard send/receive helpers

### v0.3

- ✅ KDE Connect bridge (graceful fallback when missing)
- ✅ Notification sync helpers (`kde notify`)
- ✅ Battery status (`kde battery`)
- ✅ Ring/find phone action (`kde ring`)
- ✅ Media control (`kde media`)

### v0.4

- 🚧 Tauri GUI groundwork (`gui status`)
- 🚧 Device card/pairing/profile scaffolding in stable config
- ✅ Hyprland rule generator (`gui generate-rules`)

### v1.0

- 🚧 Stable config format path (`config_version = 1`, `config status`, `config migrate`)
- 🚧 Polished GUI + tray/status
- ⏳ AUR + Nix + distro packaging
- ⏳ Dedicated docs site

## Contributing

Contributions are welcome:

1. Open an issue with your use-case/problem.
2. Keep changes focused and testable.
3. Prefer practical CLI UX improvements.
4. Include docs/examples updates when behavior changes.

## Showcase Blurb

Built `hypr-phone`, an open-source Wayland-native Android companion for Hyprland that unifies ADB device management, wireless pairing, scrcpy screen mirroring, special workspace integration, status module output, and rofi/wofi quick actions into a polished Linux workflow.

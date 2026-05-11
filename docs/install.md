# Install

This guide covers the hypr-phone v0.1 MVP setup on Arch Linux + Hyprland.

> v0.1 is CLI-first (no GUI yet).

## 1) Install dependencies (Arch)

Required:

```bash
sudo pacman -S android-tools scrcpy wl-clipboard libnotify rofi
```

Optional:

```bash
sudo pacman -S kdeconnect
sudo pacman -S wofi
```

## 2) Build hypr-phone

```bash
git clone https://github.com/<you>/hypr-phone.git
cd hypr-phone
cargo build --release
```

Use the binary directly:

```bash
./target/release/hypr-phone --help
```

Optional: install to local bin:

```bash
install -Dm755 ./target/release/hypr-phone ~/.local/bin/hypr-phone
```

## 3) Initialize config

```bash
hypr-phone config init
hypr-phone config path
```

Default config path:

```txt
~/.config/hypr-phone/config.toml
```

## 4) Verify environment

```bash
hypr-phone doctor
```

Expected checks include:

- adb
- scrcpy
- hyprctl
- wl-copy / wl-paste
- notify-send
- rofi or wofi

## 5) Add integration snippets

- Hyprland rules: `examples/hyprland.conf`
- Waybar module: `examples/waybar-module.json`
- Wayle notes: `examples/wayle-module.md`
- Launcher scripts: `examples/rofi-hypr-phone.sh`, `examples/wofi-hypr-phone.sh`

## 6) First mirror test

```bash
hypr-phone devices
hypr-phone mirror --profile default
```

If this fails, see [docs/troubleshooting.md](troubleshooting.md).

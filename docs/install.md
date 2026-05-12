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
sudo pacman -S qrencode
```

## 2) Build hypr-phone

```bash
git clone https://github.com/<you>/hypr-phone.git
cd hypr-phone
cargo build --release
```

## 3) Install globally (user-local)

```bash
./scripts/install-global.sh
```

This script:

- Runs `cargo install --path . --force` from the project root
- Ensures `~/.local/bin` exists
- Creates/updates:

```txt
~/.local/bin/hypr-phone -> ${CARGO_HOME:-$HOME/.cargo}/bin/hypr-phone
```

Quick check:

```bash
~/.local/bin/hypr-phone --help
```

## 4) Ensure PATH includes local bins

Add this to your shell rc (`~/.bashrc`, `~/.zshrc`) if needed:

```bash
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
```

Then reload your shell and verify:

```bash
command -v hypr-phone
```

## 5) Initialize config

```bash
hypr-phone config init
hypr-phone config path
```

Default config path:

```txt
~/.config/hypr-phone/config.toml
```

## 6) Hyprland integration (apply now)

```bash
mkdir -p ~/.config/hypr
cp ./examples/hyprland.conf ~/.config/hypr/hypr-phone.conf
grep -qxF 'source = ~/.config/hypr/hypr-phone.conf' ~/.config/hypr/hyprland.conf || echo 'source = ~/.config/hypr/hypr-phone.conf' >> ~/.config/hypr/hyprland.conf
hyprctl reload
```

## 7) Verify environment

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

## 8) First mirror test

```bash
hypr-phone devices
hypr-phone mirror --profile default
```

If this fails, see [docs/troubleshooting.md](troubleshooting.md).

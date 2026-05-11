# Troubleshooting

This page focuses on practical fixes for hypr-phone v0.1 on Arch + Hyprland.

## First triage commands

```bash
hypr-phone doctor
hypr-phone devices
adb devices -l
```

## `adb was not found`

Install:

```bash
sudo pacman -S android-tools
```

Then verify:

```bash
which adb
hypr-phone doctor
```

## `scrcpy failed to start`

Check:

1. Device is authorized for ADB.
2. `adb devices` shows `device` (not `unauthorized`/`offline`).
3. scrcpy is installed.

Try:

```bash
hypr-phone doctor
hypr-phone devices
scrcpy --version
```

Install/reinstall if needed:

```bash
sudo pacman -S scrcpy
```

## Wireless pairing issues

Use exact pair endpoint and code from Android wireless debugging:

```bash
hypr-phone pair 192.168.1.23:37123 123456
hypr-phone connect 192.168.1.23:5555
```

If connect fails:

- Confirm phone and PC are on same network.
- Ensure phone wireless debugging remains enabled.
- Re-run pair (pair endpoint can rotate).

## Device shows `unauthorized`

On phone:

- Accept RSA prompt.
- If prompt is missing, revoke USB debugging authorizations and reconnect.

On desktop:

```bash
adb kill-server
adb start-server
adb devices
```

## Hyprland rules not applied

Mirror may still work even if window automation fails.

Checklist:

1. `hyprctl` exists:
   ```bash
   which hyprctl
   ```
2. Window title rule matches `hypr-phone: ...`.
3. Use example snippet from `examples/hyprland.conf`.
4. Reload Hyprland config:
   ```bash
   hyprctl reload
   ```

## `hypr-phone module` not updating in bar

Use `return-type: json` and a short interval in Waybar config.
Example:

```json
{
  "custom/hypr-phone": {
    "exec": "hypr-phone module",
    "return-type": "json",
    "interval": 5,
    "on-click": "hypr-phone menu",
    "tooltip": true
  }
}
```

## rofi/wofi menu does not open

Install one launcher:

```bash
sudo pacman -S rofi
# or
sudo pacman -S wofi
```

If both are installed, rofi is usually preferred unless config/script overrides it.

## Still stuck?

Collect output for issue reports:

```bash
hypr-phone doctor
hypr-phone devices --json
hypr-phone module
```

Include:

- Hyprland version
- Arch package versions (`adb`, `scrcpy`, `rofi`/`wofi`)
- Exact command and full error output

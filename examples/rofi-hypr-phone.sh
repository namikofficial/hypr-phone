#!/usr/bin/env bash
set -euo pipefail

if ! command -v hypr-phone >/dev/null 2>&1; then
  notify-send "hypr-phone" "hypr-phone not found in PATH"
  exit 1
fi

if ! command -v rofi >/dev/null 2>&1; then
  notify-send "hypr-phone" "rofi is not installed"
  exit 1
fi

menu_entries=$(
  cat <<'EOF'
Mirror default device
Mirror low latency
List devices
Pair wireless ADB
Connect wireless ADB
Disconnect device
Open ADB shell
Copy phone screenshot
Push file to phone
EOF
)

choice=$(printf '%s\n' "$menu_entries" | rofi -dmenu -i -p "hypr-phone")
[[ -z "${choice:-}" ]] && exit 0

case "$choice" in
  "Mirror default device")
    hypr-phone mirror --profile default
    ;;
  "Mirror low latency")
    hypr-phone mirror --profile low_latency
    ;;
  "List devices")
    hypr-phone devices | rofi -dmenu -i -p "Connected devices (Esc to close)" >/dev/null || true
    ;;
  "Pair wireless ADB")
    endpoint=$(rofi -dmenu -p "Pair endpoint (ip:port)")
    [[ -z "${endpoint:-}" ]] && exit 0
    code=$(rofi -dmenu -p "Pairing code")
    [[ -z "${code:-}" ]] && exit 0
    hypr-phone pair "$endpoint" "$code"
    ;;
  "Connect wireless ADB")
    endpoint=$(rofi -dmenu -p "Connect endpoint (ip:port)")
    [[ -z "${endpoint:-}" ]] && exit 0
    hypr-phone connect "$endpoint"
    ;;
  "Disconnect device")
    serial=$(rofi -dmenu -p "Serial to disconnect")
    [[ -z "${serial:-}" ]] && exit 0
    hypr-phone disconnect "$serial"
    ;;
  "Open ADB shell")
    serial=$(rofi -dmenu -p "Serial (leave empty for default)")
    if [[ -n "${serial:-}" ]]; then
      exec foot -e adb -s "$serial" shell
    else
      exec foot -e adb shell
    fi
    ;;
  "Copy phone screenshot")
    out="${HOME}/Pictures/hypr-phone-$(date +%Y%m%d-%H%M%S).png"
    adb exec-out screencap -p >"$out"
    wl-copy <"$out"
    notify-send "hypr-phone" "Screenshot saved and copied: $out"
    ;;
  "Push file to phone")
    source_file=$(rofi -dmenu -p "Local file path")
    [[ -z "${source_file:-}" ]] && exit 0
    remote_path=$(rofi -dmenu -p "Remote path (default /sdcard/Download/)")
    remote_path=${remote_path:-/sdcard/Download/}
    adb push "$source_file" "$remote_path"
    ;;
esac

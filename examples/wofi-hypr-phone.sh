#!/usr/bin/env bash
set -euo pipefail

if ! command -v hypr-phone >/dev/null 2>&1; then
  notify-send "hypr-phone" "hypr-phone not found in PATH"
  exit 1
fi

if ! command -v wofi >/dev/null 2>&1; then
  notify-send "hypr-phone" "wofi is not installed"
  exit 1
fi

select_wofi() {
  wofi --dmenu --prompt "$1"
}

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

choice=$(printf '%s\n' "$menu_entries" | select_wofi "hypr-phone")
[[ -z "${choice:-}" ]] && exit 0

case "$choice" in
  "Mirror default device")
    hypr-phone mirror --profile default
    ;;
  "Mirror low latency")
    hypr-phone mirror --profile low_latency
    ;;
  "List devices")
    hypr-phone devices | select_wofi "Connected devices (Esc to close)" >/dev/null || true
    ;;
  "Pair wireless ADB")
    endpoint=$(printf '' | select_wofi "Pair endpoint (ip:port)")
    [[ -z "${endpoint:-}" ]] && exit 0
    code=$(printf '' | select_wofi "Pairing code")
    [[ -z "${code:-}" ]] && exit 0
    hypr-phone pair "$endpoint" "$code"
    ;;
  "Connect wireless ADB")
    endpoint=$(printf '' | select_wofi "Connect endpoint (ip:port)")
    [[ -z "${endpoint:-}" ]] && exit 0
    hypr-phone connect "$endpoint"
    ;;
  "Disconnect device")
    serial=$(printf '' | select_wofi "Serial to disconnect")
    [[ -z "${serial:-}" ]] && exit 0
    hypr-phone disconnect "$serial"
    ;;
  "Open ADB shell")
    serial=$(printf '' | select_wofi "Serial (leave empty for default)")
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
    source_file=$(printf '' | select_wofi "Local file path")
    [[ -z "${source_file:-}" ]] && exit 0
    remote_path=$(printf '' | select_wofi "Remote path (default /sdcard/Download/)")
    remote_path=${remote_path:-/sdcard/Download/}
    adb push "$source_file" "$remote_path"
    ;;
esac

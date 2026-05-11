# Wayle Integration

`hypr-phone module` outputs compact JSON with phone connection state.

Expected output:

```json
{
  "text": "󰄜 Pixel",
  "tooltip": "Android device connected: Pixel",
  "class": "connected"
}
```

Use this output in a Wayle custom module or adapter.

The module command:

```bash
hypr-phone module
```

Recommended click action:

```bash
hypr-phone menu
```

Notes:

- Keep module polling fast (for example every 3-5 seconds).
- `hypr-phone module` should be non-blocking and should never launch `scrcpy`.
- Use the returned `class` field (`connected` / `disconnected`) for styling.

#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

CARGO_HOME_DIR=${CARGO_HOME:-"$HOME/.cargo"}
CARGO_BIN_PATH="$CARGO_HOME_DIR/bin/hypr-phone"
LOCAL_BIN_DIR="$HOME/.local/bin"
LOCAL_LINK_PATH="$LOCAL_BIN_DIR/hypr-phone"

cd "$PROJECT_ROOT"

echo "Installing hypr-phone from $PROJECT_ROOT ..."
cargo install --path . --force

mkdir -p "$LOCAL_BIN_DIR"

if [ ! -x "$CARGO_BIN_PATH" ]; then
  echo "Error: expected executable not found at $CARGO_BIN_PATH" >&2
  exit 1
fi

ln -sf "$CARGO_BIN_PATH" "$LOCAL_LINK_PATH"

if [ ! -x "$LOCAL_LINK_PATH" ]; then
  echo "Error: symlink target is not executable: $LOCAL_LINK_PATH" >&2
  exit 1
fi

echo
echo "hypr-phone global install complete."
echo "  Installed binary: $CARGO_BIN_PATH"
echo "  Symlink created:  $LOCAL_LINK_PATH"
echo
echo "Quick verify:"
echo "  $LOCAL_LINK_PATH --help"

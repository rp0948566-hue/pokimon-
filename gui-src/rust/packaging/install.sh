#!/usr/bin/env bash
# Per-user install of RetinaCare: binary, launcher, icon, menu entry and desktop shortcut. No sudo.
#   ./packaging/install.sh            install or update
#   ./packaging/install.sh uninstall  remove (keeps patient data and models)
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$HOME/.local/share/retinacare"
APPS="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
CONF="${XDG_CONFIG_HOME:-$HOME/.config}/retinacare"
DESK="$(xdg-user-dir DESKTOP 2>/dev/null || echo "$HOME/Desktop")"

if [ "${1:-}" = uninstall ]; then
  rm -rf "$APP" "$APPS/retinacare.desktop" "$ICON_DIR/retinacare.png" "$DESK/RetinaCare.desktop"
  update-desktop-database "$APPS" 2>/dev/null || true
  echo "Removed. Kept: ~/.local/share/retinacare-data style folders (patients), $CONF, models."
  exit 0
fi

REL="$ROOT/target/release"
[ -x "$REL/retinacare" ] || { echo "Build first: cargo build --release" >&2; exit 1; }

# Application files only; the patient database lives elsewhere (XDG data dir) and is never touched here.
mkdir -p "$APP/bin" "$APPS" "$ICON_DIR" "$CONF"
install -m 755 "$REL/retinacare" "$APP/bin/retinacare"
install -m 755 "$ROOT/packaging/retinacare-launch" "$APP/bin/retinacare-launch"
for so in "$REL"/libonnxruntime_providers_*.so; do install -m 644 "$so" "$APP/bin/"; done
install -m 644 "$ROOT/../icon/retinacare.png" "$ICON_DIR/retinacare.png"

# Runtime locations are detected once and written to launcher.env; edit that file to move them.
if [ ! -f "$CONF/launcher.env" ]; then
  CUDA=""
  for d in "$HOME/Desktop/retinacare-cuda/libs13/nvidia/cu13/lib" "$HOME/Desktop/retinacare-cuda/libs13/nvidia/cudnn/lib"; do
    [ -d "$d" ] && CUDA="${CUDA:+$CUDA:}$d"
  done
  {
    echo "# Written by install.sh. RetinaCare launcher settings."
    echo "RETINACARE_MODELS_DIR=$HOME/Desktop/Dird/models"
    echo "CUDA_LIB_DIRS=$CUDA"
    [ -x "$HOME/ollama-user/bin/ollama" ] && echo "OLLAMA_BIN=$HOME/ollama-user/bin/ollama"
    [ -d /usr/share/ollama/.ollama/models ] && echo "OLLAMA_MODELS=/usr/share/ollama/.ollama/models"
  } >"$CONF/launcher.env"
fi

cat >"$APPS/retinacare.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=RetinaCare
GenericName=Retinal screening research suite
Comment=Local AI retinal image analysis (research prototype)
Exec=$APP/bin/retinacare-launch
Icon=retinacare
Terminal=false
Categories=Science;Education;MedicalSoftware;
StartupWMClass=retinacare
StartupNotify=true
EOF
chmod 644 "$APPS/retinacare.desktop"
update-desktop-database "$APPS" 2>/dev/null || true
gtk-update-icon-cache -q -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

# Desktop shortcut: a copy of the same entry, marked trusted for GNOME with the user's own gio command.
if [ -d "$DESK" ]; then
  cp "$APPS/retinacare.desktop" "$DESK/RetinaCare.desktop"
  chmod 755 "$DESK/RetinaCare.desktop"
  gio set "$DESK/RetinaCare.desktop" metadata::trusted true 2>/dev/null || true
fi
echo "Installed to $APP"

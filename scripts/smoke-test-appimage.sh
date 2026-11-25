#!/usr/bin/env bash
set -euo pipefail

appimage=${1:?Usage: smoke-test-appimage.sh <AppImage> [log-file]}
log_file=${2:-"${TMPDIR:-/tmp}/dofus-switcher-smoke.log"}

if [[ ! -x "$appimage" ]]; then
  echo "AppImage is missing or not executable: $appimage" >&2
  exit 1
fi
appimage=$(realpath -- "$appimage")

xvfb-run --auto-servernum \
  --server-args="-screen 0 1024x768x24 +extension GLX +render -noreset" \
  bash -euo pipefail -c '
    appimage=$1
    log_file=$2
    setsid "$appimage" --appimage-extract-and-run >"$log_file" 2>&1 &
    app_pid=$!

    cleanup() {
      kill -TERM -- "-$app_pid" 2>/dev/null || true
      wait "$app_pid" 2>/dev/null || true
    }
    trap cleanup EXIT

    for _ in {1..40}; do
      if xdotool search --onlyvisible --name "Dofus Switcher" >/dev/null 2>&1; then
        echo "AppImage opened its main window under Xvfb."
        exit 0
      fi
      if ! kill -0 "$app_pid" 2>/dev/null; then
        if wait "$app_pid"; then
          status=0
        else
          status=$?
        fi
        cat "$log_file"
        echo "AppImage exited before its window appeared (status $status)." >&2
        exit 1
      fi
      sleep 0.25
    done

    cat "$log_file"
    echo "Timed out waiting for the Dofus Switcher window." >&2
    exit 1
  ' _ "$appimage" "$log_file"

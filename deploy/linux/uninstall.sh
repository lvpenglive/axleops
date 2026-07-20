#!/usr/bin/env bash
# Remove AxleOps systemd units (and optionally install files).
#
#   sudo ./deploy/linux/uninstall.sh agent
#   sudo ./deploy/linux/uninstall.sh all --purge --prefix /opt/axleops
#
set -euo pipefail

PREFIX="/opt/axleops"
PURGE=0
COMPONENT=""

usage() {
  cat <<'EOF'
Usage: uninstall.sh <agent|admin|proxy|all> [options]

Options:
  --prefix DIR   Install root (default: /opt/axleops)
  --purge        Also delete PREFIX/<component> (keeps data if you copy it first)
  -h, --help
EOF
}

die() { echo "error: $*" >&2; exit 1; }

[[ "${EUID:-$(id -u)}" -eq 0 ]] || die "must run as root"

while [[ $# -gt 0 ]]; do
  case "$1" in
    agent|admin|proxy|all) COMPONENT="$1"; shift ;;
    --prefix) PREFIX="${2:?}"; shift 2 ;;
    --purge) PURGE=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done

[[ -n "$COMPONENT" ]] || { usage; exit 1; }

remove_one() {
  local name="$1"
  local unit="axleops-${name}.service"
  echo "==> uninstall ${name}"
  systemctl disable --now "$unit" 2>/dev/null || true
  rm -f "/etc/systemd/system/${unit}"
  if [[ "$PURGE" -eq 1 ]]; then
    echo "    purge ${PREFIX}/${name}"
    rm -rf "${PREFIX:?}/${name}"
  fi
}

case "$COMPONENT" in
  all)
    remove_one agent
    remove_one admin
    remove_one proxy
    ;;
  *)
    remove_one "$COMPONENT"
    ;;
esac

systemctl daemon-reload
echo "done."

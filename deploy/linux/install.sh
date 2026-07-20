#!/usr/bin/env bash
# Install AxleOps component(s) under PREFIX and register systemd units.
# Run as root. Typical usage from a release package root:
#
#   sudo ./deploy/linux/install.sh agent --start
#   sudo ./deploy/linux/install.sh admin --start
#   sudo ./deploy/linux/install.sh all --prefix /opt/axleops --start
#
set -euo pipefail

PREFIX="/opt/axleops"
USER_NAME="axleops"
GROUP_NAME="axleops"
DO_START=0
COMPONENT=""
SOURCE=""

usage() {
  cat <<'EOF'
Usage: install.sh <agent|admin|proxy|all> [options]

Options:
  --prefix DIR     Install root (default: /opt/axleops)
  --from DIR       Package / source root that contains agent/ admin/ proxy/
                   (default: parent of deploy/, i.e. package root)
  --user NAME      Run-as user (default: axleops; use root to skip dedicated user)
  --group NAME     Run-as group (default: same as --user)
  --start          systemctl enable --now after install
  -h, --help       Show this help

Examples:
  sudo ./deploy/linux/install.sh agent --start
  sudo ./deploy/linux/install.sh admin --prefix /opt/axleops --start
  sudo ./deploy/linux/install.sh all --user root --start
EOF
}

die() { echo "error: $*" >&2; exit 1; }

need_root() {
  if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
    die "must run as root (try sudo)"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    agent|admin|proxy|all)
      COMPONENT="$1"
      shift
      ;;
    --prefix)
      PREFIX="${2:?}"
      shift 2
      ;;
    --from)
      SOURCE="${2:?}"
      shift 2
      ;;
    --user)
      USER_NAME="${2:?}"
      GROUP_NAME="${2:?}"
      shift 2
      ;;
    --group)
      GROUP_NAME="${2:?}"
      shift 2
      ;;
    --start)
      DO_START=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1 (see --help)"
      ;;
  esac
done

[[ -n "$COMPONENT" ]] || { usage; exit 1; }
need_root

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -z "$SOURCE" ]]; then
  # deploy/linux -> package root (or repo root)
  SOURCE="$(cd "${SCRIPT_DIR}/../.." && pwd)"
fi
SOURCE="$(cd "$SOURCE" && pwd)"

render_unit() {
  local src="$1"
  local dest="$2"
  sed \
    -e "s|{{PREFIX}}|${PREFIX}|g" \
    -e "s|{{USER}}|${USER_NAME}|g" \
    -e "s|{{GROUP}}|${GROUP_NAME}|g" \
    "$src" >"$dest"
}

ensure_user() {
  if [[ "$USER_NAME" == "root" ]]; then
    return 0
  fi
  if ! getent group "$GROUP_NAME" >/dev/null 2>&1; then
    groupadd --system "$GROUP_NAME"
  fi
  if ! id -u "$USER_NAME" >/dev/null 2>&1; then
    useradd --system --gid "$GROUP_NAME" --home-dir "$PREFIX" \
      --shell /usr/sbin/nologin --comment "AxleOps" "$USER_NAME" 2>/dev/null \
      || useradd --system --gid "$GROUP_NAME" --home-dir "$PREFIX" \
        --shell /sbin/nologin --comment "AxleOps" "$USER_NAME"
  fi
  mkdir -p "$PREFIX"
  chown "${USER_NAME}:${GROUP_NAME}" "$PREFIX"
}

# Resolve binary + config + static from release package layout or repo layout.
resolve_paths() {
  local name="$1"
  local bin="axleops-${name}"
  BIN_SRC=""
  CFG_DIR=""
  STATIC_SRC=""

  if [[ -f "${SOURCE}/${name}/${bin}" ]]; then
    BIN_SRC="${SOURCE}/${name}/${bin}"
    CFG_DIR="${SOURCE}/${name}"
    STATIC_SRC="${SOURCE}/${name}/static"
  elif [[ -f "${SOURCE}/axleops-${name}/target/release/${bin}" ]]; then
    BIN_SRC="${SOURCE}/axleops-${name}/target/release/${bin}"
    CFG_DIR="${SOURCE}/axleops-${name}"
    STATIC_SRC="${SOURCE}/axleops-${name}/static"
  else
    die "binary not found for ${name}. Expected ${SOURCE}/${name}/${bin} or repo target/release/${bin}"
  fi
}

install_one() {
  local name="$1"
  local bin="axleops-${name}"
  local dest_dir="${PREFIX}/${name}"
  local unit_src="${SCRIPT_DIR}/axleops-${name}.service"
  local unit_dest="/etc/systemd/system/axleops-${name}.service"

  resolve_paths "$name"
  [[ -f "$unit_src" ]] || die "unit template missing: $unit_src"

  echo "==> install ${name} -> ${dest_dir}"
  mkdir -p "$dest_dir"
  install -m 0755 "$BIN_SRC" "${dest_dir}/${bin}"

  if [[ "$name" == "admin" ]]; then
    [[ -d "$STATIC_SRC" ]] || die "admin static/ missing: ${STATIC_SRC}"
    mkdir -p "${dest_dir}/static"
    cp -a "${STATIC_SRC}/." "${dest_dir}/static/"
  fi

  if [[ ! -f "${dest_dir}/config.toml" ]]; then
    if [[ -f "${CFG_DIR}/config.toml" ]]; then
      cp "${CFG_DIR}/config.toml" "${dest_dir}/config.toml"
    elif [[ -f "${CFG_DIR}/config.example.toml" ]]; then
      cp "${CFG_DIR}/config.example.toml" "${dest_dir}/config.toml"
      echo "    wrote ${dest_dir}/config.toml from example — edit secrets before start"
    else
      die "no config.toml or config.example.toml in ${CFG_DIR}"
    fi
  else
    echo "    keep existing ${dest_dir}/config.toml"
  fi

  mkdir -p "${dest_dir}/data"
  if [[ "$USER_NAME" != "root" ]]; then
    chown -R "${USER_NAME}:${GROUP_NAME}" "$dest_dir"
  fi
  chmod 600 "${dest_dir}/config.toml" 2>/dev/null || true

  render_unit "$unit_src" "$unit_dest"
  echo "    unit ${unit_dest}"

  if [[ "$DO_START" -eq 1 ]]; then
    systemctl daemon-reload
    systemctl enable --now "axleops-${name}.service"
    systemctl --no-pager --full status "axleops-${name}.service" || true
  else
    systemctl daemon-reload
    systemctl enable "axleops-${name}.service"
    echo "    enabled; start with: systemctl start axleops-${name}"
  fi
}

command -v systemctl >/dev/null 2>&1 || die "systemctl not found (systemd required)"

ensure_user

case "$COMPONENT" in
  all)
    install_one agent
    install_one admin
    install_one proxy
    ;;
  agent|admin|proxy)
    install_one "$COMPONENT"
    ;;
esac

echo "done. prefix=${PREFIX} user=${USER_NAME}"
if [[ "$COMPONENT" == "agent" || "$COMPONENT" == "all" ]] && [[ "$USER_NAME" != "root" ]]; then
  echo "note: agent runs as ${USER_NAME} — grant it read/exec on JAR/script paths it manages."
fi

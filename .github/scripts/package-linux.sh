#!/usr/bin/env bash
# Package AxleOps release binaries into dist/<PACKAGE_NAME>.tar.gz
set -euo pipefail

PACKAGE_NAME="${PACKAGE_NAME:?PACKAGE_NAME is required}"
PACKAGE_LABEL="${PACKAGE_LABEL:-$PACKAGE_NAME}"

STAGE="dist/${PACKAGE_NAME}"
mkdir -p "${STAGE}/agent" "${STAGE}/admin/static" "${STAGE}/proxy" "${STAGE}/deploy"

cp axleops-agent/target/release/axleops-agent "${STAGE}/agent/"
cp axleops-agent/config.example.toml "${STAGE}/agent/config.example.toml"

cp axleops-admin/target/release/axleops-admin "${STAGE}/admin/"
cp axleops-admin/config.example.toml "${STAGE}/admin/config.example.toml"
cp -r axleops-admin/static/* "${STAGE}/admin/static/"

cp axleops-proxy/target/release/axleops-proxy "${STAGE}/proxy/"
cp axleops-proxy/config.example.toml "${STAGE}/proxy/config.example.toml"

# Service install helpers (systemd + Windows + Docker recipes)
cp -a deploy/. "${STAGE}/deploy/"
chmod +x "${STAGE}/deploy/linux/install.sh" "${STAGE}/deploy/linux/uninstall.sh" 2>/dev/null || true
chmod +x "${STAGE}/deploy/docker/up.sh" 2>/dev/null || true

printf '%s\n' \
  "AxleOps package: ${PACKAGE_LABEL}" \
  '' \
  '1) Copy config.example.toml -> config.toml in each component dir and edit secrets.' \
  '2) Manual start (from that directory so ./data and ./static resolve):' \
  '' \
  '   cd agent && ./axleops-agent' \
  '   cd admin && ./axleops-admin' \
  '   cd proxy && ./axleops-proxy   # optional' \
  '' \
  '3) Or install as OS service (see deploy/README.md):' \
  '' \
  '   # Linux' \
  '   sudo ./deploy/linux/install.sh agent --start' \
  '   sudo ./deploy/linux/install.sh admin --start' \
  '' \
  '   # Windows (Admin PowerShell)' \
  '   .\deploy\windows\install.ps1 -Component agent -Start' \
  '   .\deploy\windows\install.ps1 -Component admin -Start' \
  '' \
  '4) Docker Compose needs the full git checkout (builds from source):' \
  '' \
  '   ./deploy/docker/up.sh' \
  '' \
  'Admin UI requires the static/ folder next to axleops-admin.' \
  > "${STAGE}/README.txt"

chmod +x \
  "${STAGE}/agent/axleops-agent" \
  "${STAGE}/admin/axleops-admin" \
  "${STAGE}/proxy/axleops-proxy"

mkdir -p dist
tar -C dist -czf "dist/${PACKAGE_NAME}.tar.gz" "${PACKAGE_NAME}"
ls -lh "dist/${PACKAGE_NAME}.tar.gz"
find "${STAGE}" -type f | sort

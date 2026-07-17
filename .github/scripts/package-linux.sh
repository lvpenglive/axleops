#!/usr/bin/env bash
# Package AxleOps release binaries into dist/<PACKAGE_NAME>.tar.gz
set -euo pipefail

PACKAGE_NAME="${PACKAGE_NAME:?PACKAGE_NAME is required}"
PACKAGE_LABEL="${PACKAGE_LABEL:-$PACKAGE_NAME}"

STAGE="dist/${PACKAGE_NAME}"
mkdir -p "${STAGE}/agent" "${STAGE}/admin/static" "${STAGE}/proxy"

cp axleops-agent/target/release/axleops-agent "${STAGE}/agent/"
cp axleops-agent/config.example.toml "${STAGE}/agent/config.example.toml"

cp axleops-admin/target/release/axleops-admin "${STAGE}/admin/"
cp axleops-admin/config.example.toml "${STAGE}/admin/config.example.toml"
cp -r axleops-admin/static/* "${STAGE}/admin/static/"

cp axleops-proxy/target/release/axleops-proxy "${STAGE}/proxy/"
cp axleops-proxy/config.example.toml "${STAGE}/proxy/config.example.toml"

printf '%s\n' \
  "AxleOps package: ${PACKAGE_LABEL}" \
  '' \
  '1) Copy config.example.toml -> config.toml in each component dir and edit secrets.' \
  '2) Start from that directory so relative paths (./data, ./static) work:' \
  '' \
  '   cd agent && ./axleops-agent' \
  '   cd admin && ./axleops-admin' \
  '   cd proxy && ./axleops-proxy   # optional' \
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

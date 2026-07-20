#!/usr/bin/env bash
# One-shot: create .env if missing, then build & start Admin + Agent.
# Usage (from anywhere):
#   ./deploy/docker/up.sh
#   ./deploy/docker/up.sh --profile proxy
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ENV_FILE="deploy/docker/.env"
if [[ ! -f "$ENV_FILE" ]]; then
  cp deploy/docker/.env.example "$ENV_FILE"
  echo "created ${ENV_FILE} — edit secrets, then re-run if needed"
fi

mkdir -p deploy/docker/apps

echo "building & starting (env: ${ENV_FILE})…"
docker compose --env-file "$ENV_FILE" up -d --build "$@"

echo
echo "Admin UI:  http://127.0.0.1:${ADMIN_PORT:-9000}"
echo "Agent:     http://127.0.0.1:${AGENT_PORT:-9100}/health"
echo "Register Agent in console:"
echo "  Base URL: http://agent:9100   (Docker DNS; Admin container → Agent)"
echo "         or http://127.0.0.1:${AGENT_PORT:-9100}  (from browser/host tools)"
echo "  Token:    value of AGENT_TOKEN in ${ENV_FILE}"
echo
docker compose --env-file "$ENV_FILE" ps

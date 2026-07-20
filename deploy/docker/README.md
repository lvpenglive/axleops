# Docker / Compose

## Quick start

```bash
# Linux / macOS / Git Bash
./deploy/docker/up.sh

# Windows PowerShell
.\deploy\docker\up.ps1
```

Creates `deploy/docker/.env` from `.env.example` on first run, builds images, starts **Admin + Agent**.

- Admin: http://127.0.0.1:9000  
- Agent health: http://127.0.0.1:9100/health  

In the console, register the Agent with:

| Field | Value |
|-------|--------|
| Base URL | `http://agent:9100` (Admin→Agent on the Compose network) |
| Token | `AGENT_TOKEN` from `.env` |

Login with `ADMIN_SEED_USER` / `ADMIN_SEED_PASSWORD`.

Optional Proxy:

```bash
./deploy/docker/up.sh --profile proxy
```

## Files

| Path | Role |
|------|------|
| `docker-compose.yml` (repo root) | Admin + Agent; Proxy via profile |
| `deploy/docker/Dockerfile.*` | Multi-stage Rust builds |
| `deploy/docker/.env.example` | Secrets & ports template |
| `deploy/docker/apps/` | Mounted into Agent as `/apps` for JAR/scripts |

## Notes

- Agent image includes OpenJDK 17 JRE so `jar` services can run **inside** the container. Point `jar_path` at `/apps/….jar`.
- For managing processes on the **host** OS, prefer the systemd / Windows service installers under `deploy/linux` and `deploy/windows` instead of containerized Agent.
- Change all tokens and the seed password before any shared/lab exposure.
- Stop: `docker compose --env-file deploy/docker/.env down`

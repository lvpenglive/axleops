# AxleOps service install helpers
#
# Linux (systemd):
#   sudo ./linux/install.sh agent --start
#   sudo ./linux/install.sh admin --start
#   sudo ./linux/uninstall.sh agent
#
# Windows (Administrator PowerShell):
#   .\windows\install.ps1 -Component agent -Start
#   .\windows\install.ps1 -Component admin -Start
#   # Prefer NSSM if available; otherwise registers an AtStartup Scheduled Task.
#   .\windows\uninstall.ps1 -Component agent
#
# Docker Compose (demo / lab):
#   ./docker/up.sh
#   .\docker\up.ps1
#   See docker/README.md
#
# Default install roots:
#   Linux:   /opt/axleops/{agent,admin,proxy}
#   Windows: C:\axleops\{agent,admin,proxy}
#
# Always edit config.toml / .env secrets before start on a fresh install.

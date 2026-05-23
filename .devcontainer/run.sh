#!/usr/bin/env bash
# Launch (and attach to) the wholesome-swolesome dev container.
#
# Usage:
#   ./.devcontainer/run.sh                 # interactive bash in /workspace
#   ./.devcontainer/run.sh claude --dangerously-skip-permissions
#   ./.devcontainer/run.sh trunk serve     # then visit http://localhost:8080
#
# The container is kept running in the background so multiple shells can
# attach (e.g. one for `trunk serve`, one for `claude`).

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
compose=(docker compose -f "${here}/docker-compose.yml")

# Start (or no-op if already up).
"${compose[@]}" up -d

if [[ $# -eq 0 ]]; then
  exec "${compose[@]}" exec dev bash
else
  exec "${compose[@]}" exec dev "$@"
fi

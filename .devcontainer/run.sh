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
#
# Before bringing the container up, this script refreshes the Claude CLI
# OAuth credentials from the host into ~/.claude-container/.credentials.json,
# which docker-compose bind-mounts into the container at
# /home/node/.claude/.credentials.json (read-only).

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
compose=(docker compose -f "${here}/docker-compose.yml")

# --- Refresh host-side credentials snapshot for the container -------------
cred_dir="${HOME}/.claude-container"
cred_file="${cred_dir}/.credentials.json"
mkdir -p "${cred_dir}"
chmod 700 "${cred_dir}"

extracted=""
case "$(uname -s)" in
  Darwin)
    # macOS stores Claude OAuth tokens in the login Keychain. Pull them out
    # as a JSON blob and stage it where the container can read it.
    if extracted="$(security find-generic-password \
                       -s "Claude Code-credentials" -w 2>/dev/null)"; then
      :
    else
      echo "run.sh: no 'Claude Code-credentials' entry in macOS Keychain." >&2
      echo "         Run 'claude' on the host once and complete /login first." >&2
      exit 1
    fi
    ;;
  Linux)
    # On Linux the host CLI already writes credentials to a file; just copy it.
    src="${HOME}/.claude/.credentials.json"
    if [[ -r "${src}" ]]; then
      extracted="$(cat "${src}")"
    else
      echo "run.sh: ${src} not found. Run 'claude' on the host once and /login first." >&2
      exit 1
    fi
    ;;
  *)
    echo "run.sh: unsupported host OS $(uname -s)." >&2
    exit 1
    ;;
esac

# Validate it parses as JSON before writing — guards against keychain corruption.
if ! printf '%s' "${extracted}" | python3 -c 'import json,sys; json.load(sys.stdin)' 2>/dev/null; then
  echo "run.sh: extracted credentials are not valid JSON. Refusing to write." >&2
  exit 1
fi

# Only overwrite the cached credentials file if it's missing or older than
# the Keychain copy. The mount is now read-write so the container refreshes
# OAuth access tokens in place; clobbering on every `run.sh` invocation would
# stomp those refreshes with a stale Keychain snapshot whenever the user
# hadn't recently run `claude` on the host.
should_write=1
if [[ -s "${cred_file}" ]]; then
  if newer_in_file="$(
        FILE_JSON="$(cat "${cred_file}")" \
        KEYCHAIN_JSON="${extracted}" \
        python3 - <<'PY'
import json, os, sys
try:
    f = json.loads(os.environ["FILE_JSON"]).get("claudeAiOauth", {})
    k = json.loads(os.environ["KEYCHAIN_JSON"]).get("claudeAiOauth", {})
except Exception:
    sys.exit(2)
f_exp = f.get("expiresAt") or 0
k_exp = k.get("expiresAt") or 0
# Exit 0 = file is at least as fresh as keychain; skip the write.
# Exit 1 = keychain is newer; overwrite.
sys.exit(0 if f_exp >= k_exp else 1)
PY
      )"; then
    should_write=0
  fi
fi

if (( should_write )); then
  umask 077
  printf '%s' "${extracted}" > "${cred_file}"
  chmod 600 "${cred_file}"
fi

# --- Refresh host-side gh token + git identity for the container ----------
gh_token_file="${cred_dir}/gh-token"
git_id_file="${cred_dir}/git-identity"

if command -v gh >/dev/null 2>&1 && gh auth token >/dev/null 2>&1; then
  gh auth token > "${gh_token_file}"
  chmod 600 "${gh_token_file}"
else
  echo "run.sh: 'gh' not authed on host. Run 'gh auth login' first." >&2
  exit 1
fi

# Mirror the host git identity so commits made inside the container carry
# the same name/email as on the host.
{
  name="$(git config --global --get user.name  2>/dev/null || true)"
  email="$(git config --global --get user.email 2>/dev/null || true)"
  if [[ -z "${name}" || -z "${email}" ]]; then
    echo "run.sh: missing git user.name or user.email on host." >&2
    exit 1
  fi
  printf 'GIT_USER_NAME=%s\nGIT_USER_EMAIL=%s\n' "${name}" "${email}"
} > "${git_id_file}"
chmod 600 "${git_id_file}"

# Start (or no-op if already up).
"${compose[@]}" up -d

# Wait for the entrypoint's setup (Claude symlink, gh auth, git config) to
# finish before handing off — otherwise the first `exec` can race and see a
# half-configured container.
deadline=$(( $(date +%s) + 30 ))
until "${compose[@]}" exec -T dev test -f /tmp/devcontainer-ready 2>/dev/null; do
  if (( $(date +%s) >= deadline )); then
    echo "run.sh: container setup did not complete within 30s." >&2
    "${compose[@]}" logs dev | tail -30 >&2
    exit 1
  fi
  sleep 0.2
done

if [[ $# -eq 0 ]]; then
  exec "${compose[@]}" exec dev bash
else
  exec "${compose[@]}" exec dev "$@"
fi

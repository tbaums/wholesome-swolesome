#!/usr/bin/env bash
# Devcontainer entrypoint. Runs once when the container starts (via
# `docker compose up`), before the container's main command. `docker exec`
# calls bypass this — they run their command directly.
#
# Responsibilities:
#   - Park ~/.claude.json inside the persistent ~/.claude/ named volume
#     (as .claude/claude.json) and expose it via a symlink at the path the
#     CLI expects. Without this the CLI's per-host config is lost every
#     time the container is recreated (`docker compose down && up`).

set -euo pipefail

home_json="${HOME}/.claude.json"
vol_json="${HOME}/.claude/claude.json"

# If a previous container wrote a regular .claude.json (not a symlink),
# migrate it into the persistent volume instead of throwing it away.
if [[ -f "${home_json}" && ! -L "${home_json}" ]]; then
  if [[ ! -e "${vol_json}" ]]; then
    mv "${home_json}" "${vol_json}"
  else
    # Both exist: the volume copy is authoritative. Drop the layer copy.
    rm -f "${home_json}"
  fi
fi

# Ensure the symlink points to the volume-backed file. Create an empty
# target first so the CLI doesn't warn on its first read.
if [[ ! -e "${vol_json}" ]]; then
  : > "${vol_json}"
  chmod 600 "${vol_json}"
fi
if [[ ! -L "${home_json}" || "$(readlink "${home_json}")" != ".claude/claude.json" ]]; then
  rm -f "${home_json}"
  ln -s .claude/claude.json "${home_json}"
fi

# --- gh CLI auth + git identity (provisioned by run.sh) -------------------
gh_token_src="/run/host-gh-token"
git_id_src="/run/host-git-identity"

if [[ -r "${gh_token_src}" ]]; then
  # Write ~/.config/gh/hosts.yml directly. `gh auth login --with-token`
  # hangs in a non-TTY container; this is the same end state without the
  # interactive plumbing. The token is read fresh from the host mount, so
  # rotating it on the host and re-running run.sh picks up the new value.
  token="$(tr -d '\r\n' < "${gh_token_src}")"
  user="$(GH_TOKEN="${token}" gh api user --jq .login 2>/dev/null || echo "")"
  install -d -m 0700 "${HOME}/.config/gh"
  umask 077
  cat > "${HOME}/.config/gh/hosts.yml" <<EOF
github.com:
    git_protocol: https
    user: ${user}
    oauth_token: ${token}
    users:
        ${user}:
            oauth_token: ${token}
EOF
  chmod 600 "${HOME}/.config/gh/hosts.yml"

  # Wire gh up as a git credential helper for github.com + gist.github.com,
  # mirroring what `gh auth setup-git` does. Clear any prior helper entries
  # first so we don't stack duplicates across container restarts.
  git config --global --unset-all credential.https://github.com.helper        2>/dev/null || true
  git config --global --unset-all credential.https://gist.github.com.helper   2>/dev/null || true
  git config --global credential.https://github.com.helper      "!gh auth git-credential"
  git config --global credential.https://gist.github.com.helper "!gh auth git-credential"
fi

if [[ -r "${git_id_src}" ]]; then
  # Parse the K=V file line-by-line rather than `source`-ing it: values may
  # contain spaces (e.g. "Michael Tanenbaum") which sourcing would mis-split.
  git_user_name=""
  git_user_email=""
  while IFS='=' read -r k v; do
    case "$k" in
      GIT_USER_NAME)  git_user_name="$v" ;;
      GIT_USER_EMAIL) git_user_email="$v" ;;
    esac
  done < "${git_id_src}"
  [[ -n "${git_user_name}" ]]  && git config --global user.name  "${git_user_name}"
  [[ -n "${git_user_email}" ]] && git config --global user.email "${git_user_email}"
  # Treat /workspace as a safe directory regardless of host UID mismatch.
  git config --global --add safe.directory /workspace
fi

# Sentinel for run.sh to poll: setup is complete and any subsequent
# `docker exec` will see the authed gh / git config state.
: > /tmp/devcontainer-ready

exec "$@"

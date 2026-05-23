#!/usr/bin/env bash
# Manual coach run. Pipes the prompt + state into `claude` and pushes the result.
#
# Requires:
#   - `gh` authed against both the data repo and the app repo
#   - `claude` CLI installed (Claude Code)
#   - `jq`, `curl`, `base64`
#
# Usage:
#   scripts/coach/coach.sh                  # plan for tomorrow
#   WS_TARGET_DATE=2026-05-25 scripts/coach/coach.sh
#
# Env overrides (defaults shown):
#   WS_DATA_REPO=tbaums/wholesome-swolesome-data
#   WS_DATA_BRANCH=main
#   WS_DATA_PATH=state.json
#   WS_APP_REPO=tbaums/wholesome-swolesome

set -euo pipefail

WS_DATA_REPO="${WS_DATA_REPO:-tbaums/wholesome-swolesome-data}"
WS_DATA_BRANCH="${WS_DATA_BRANCH:-main}"
WS_DATA_PATH="${WS_DATA_PATH:-state.json}"
WS_APP_REPO="${WS_APP_REPO:-tbaums/wholesome-swolesome}"
WS_TARGET_DATE="${WS_TARGET_DATE:-$(date -v+1d +%Y-%m-%d 2>/dev/null || date -d 'tomorrow' +%Y-%m-%d)}"

echo "[coach] target date: $WS_TARGET_DATE"
echo "[coach] data repo:   $WS_DATA_REPO ($WS_DATA_BRANCH/$WS_DATA_PATH)"
echo "[coach] app repo:    $WS_APP_REPO"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# ── Fetch state.json + sha ───────────────────────────────────────────────────
echo "[coach] fetching state.json..."
gh api "/repos/$WS_DATA_REPO/contents/$WS_DATA_PATH?ref=$WS_DATA_BRANCH" > "$TMP/contents.json"
jq -r '.sha' "$TMP/contents.json" > "$TMP/state.sha"
jq -r '.content' "$TMP/contents.json" | base64 -d > "$TMP/state.json"
echo "[coach] state.sha: $(cat "$TMP/state.sha")"

# ── Fetch library ────────────────────────────────────────────────────────────
echo "[coach] fetching library..."
curl -fsSL "https://raw.githubusercontent.com/$WS_APP_REPO/main/public/data/exercises.json" -o "$TMP/exercises.json"
echo "[coach] library entries: $(jq 'length' "$TMP/exercises.json")"

# ── Compose the brief and call Claude ────────────────────────────────────────
BRIEF="$TMP/brief.md"
{
  echo "# Coach Task — generate workout for $WS_TARGET_DATE"
  echo
  echo "## Current state (state.json)"
  echo '```json'
  cat "$TMP/state.json"
  echo
  echo '```'
  echo
  cat "$(dirname "$0")/PROMPT.md"
} > "$BRIEF"

echo "[coach] invoking claude (this may take 30-90s)..."
RESPONSE="$TMP/response.json"
# `claude -p` prints the response and exits; we strip markdown fences if any.
claude -p "$(cat "$BRIEF")" > "$TMP/raw_response.txt"

# Extract JSON between first { and last }
python3 - <<'PY' "$TMP/raw_response.txt" "$RESPONSE"
import re, sys, json
src = open(sys.argv[1]).read()
m = re.search(r"\{[\s\S]*\}", src)
if not m:
    sys.stderr.write("No JSON object found in Claude response:\n" + src + "\n")
    sys.exit(1)
data = json.loads(m.group(0))
json.dump(data, open(sys.argv[2], "w"), indent=2)
PY

echo "[coach] parsed workout:"
jq '.name, .rationale, (.exercises | length)' "$RESPONSE"

# ── Build ScheduledWorkout, merge, push ──────────────────────────────────────
NOW=$(date -u +%Y-%m-%dT%H:%M:%S.000Z)
UUID=$(uuidgen | tr 'A-Z' 'a-z')

jq --arg id "$UUID" --arg date "$WS_TARGET_DATE" --arg ts "$NOW" \
  '{
    id: $id,
    date: $date,
    name: .name,
    rationale: (.rationale // ""),
    source: "Coach",
    exercises: .exercises,
    created_at: $ts
  }' "$RESPONSE" > "$TMP/workout.json"

jq --slurpfile w "$TMP/workout.json" --arg date "$WS_TARGET_DATE" --arg ts "$NOW" \
  '.scheduled_workouts = ((.scheduled_workouts // []) | map(select(.date != $date)) + $w)
   | .updated_at = $ts
   | .schema_version = 2' \
  "$TMP/state.json" > "$TMP/state.next.json"

CONTENT=$(base64 < "$TMP/state.next.json" | tr -d '\n')
SHA=$(cat "$TMP/state.sha")

echo "[coach] pushing back to $WS_DATA_REPO..."
gh api -X PUT "/repos/$WS_DATA_REPO/contents/$WS_DATA_PATH" \
  -f message="coach: plan workout for $WS_TARGET_DATE" \
  -f branch="$WS_DATA_BRANCH" \
  -f sha="$SHA" \
  -f content="$CONTENT" \
  --jq '.commit.sha'

echo "[coach] ✓ done — workout for $WS_TARGET_DATE pushed."

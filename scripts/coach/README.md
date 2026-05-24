# Coach — nightly workout planner

This directory holds the agent that generates your next workout based on:
1. Your goals (from `state.json`)
2. Your recent training history (from `state.json`)
3. The exercise library (`public/data/exercises.json`)
4. Recovery science (per-muscle days-since-worked)

## Files

| File | Purpose |
|---|---|
| `PROMPT.md` | The Claude agent prompt — the contract for "given state + library, plan a workout". |
| `coach.sh`  | Bash wrapper that pulls state, calls `claude`, parses the JSON, and pushes back via `gh`. |

## Manual run

```bash
# Plan for tomorrow
scripts/coach/coach.sh

# Plan for a specific date
WS_TARGET_DATE=2026-06-01 scripts/coach/coach.sh

# Override repo defaults
WS_DATA_REPO=my-handle/my-fitness-data scripts/coach/coach.sh
```

Requires `gh` (authed), `claude` CLI, `jq`, `curl`, `python3`, and `base64` on the PATH.

## In-app manual run (no Cowork needed)

Open the app → tap **🧠 Generate workout with Claude** on the home screen.
That opens the "Coach Brief" view, which renders the same markdown packet
the nightly agent would feed Claude. Copy it, paste into a Claude Code chat,
ask for the JSON response, paste it back into the textarea, hit Import.

This is the fallback for when the nightly agent didn't run.

## Scheduling nightly runs in Cowork

Use the `/schedule` skill in your Cowork environment. Suggested invocation:

```
/schedule create
  name: "Wholesome Swolesome nightly coach"
  cron: "0 5 * * *"          # 05:00 UTC = midnight EST
  prompt: |
    Run scripts/coach/coach.sh from the wholesome-swolesome repo.
    Plan tomorrow's workout (default behaviour).
    If the script fails, post the error and exit 1.
```

Adjust the cron expression to your local midnight. Cowork will run this as
you, with your `gh` auth, so it can read/write the private data repo.

> ⚠️ You (the human) need to run the `/schedule` command yourself —
> agents like Claude Code can't create remote routines on your behalf.

## Verifying a run

After a scheduled execution:

1. `gh api /repos/$WS_DATA_REPO/commits/$WS_DATA_BRANCH --jq '.commit.message'`
   should show `coach: plan workout for YYYY-MM-DD`.
2. Open the app — the "Today" card (next morning) should show the new workout.

## Debugging

```bash
# Dry-run: build the brief without pushing
WS_DATA_REPO=my-handle/my-fitness-data bash -x scripts/coach/coach.sh
# Inspect intermediate artifacts in $TMP (printed at top of run)
```

If Claude returns malformed JSON the wrapper exits with a parse error and
prints the raw response. Re-run; if persistent, tighten the response-format
section of `PROMPT.md`.

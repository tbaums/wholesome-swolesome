# CLAUDE.md

Guidance for Claude Code (and other agents) working in this repo.

## What this is

A mobile-first PWA workout tracker built in Rust + Leptos (WASM), deployed to GitHub Pages. Target runtime is iPhone Safari (Add to Home Screen). All state lives in `localStorage`; optional GitHub-repo sync provides cross-device persistence.

Instead of a hand-edited static plan, each day's workout is **AI-generated** from the user's training goals + recent history + per-muscle recovery state. The generation happens either in-app on demand (copy/paste into a Claude Code chat) or via a nightly script that's typically scheduled in Cowork. See `README.md` for the user-facing flow and `scripts/coach/README.md` for the coach loop.

## Commands

```bash
# First-time setup
rustup target add wasm32-unknown-unknown
cargo install trunk

# Dev (auto-rebuild + auto-reload)
trunk serve                               # http://localhost:8081

# Production
trunk build --release
trunk build --release --public-url /wholesome-swolesome/   # for GitHub Pages

# Tests
wasm-pack test --headless --firefox --lib                          # Rust unit tests
npx playwright test                                                 # CI's iPhone 15 / WebKit
npx playwright test --config=playwright.local-chromium.config.ts    # dev container (no WebKit deps)
npx playwright test --config=playwright.walkthrough.config.ts       # screenshot walkthroughs
```

Important: when iterating on Playwright tests, trunk's live-reload will reload the page mid-test and break things. Both the standard and walkthrough configs pass `--no-autoreload` when they start trunk themselves; if you start trunk manually, do the same.

## Architecture

### Tech stack
- **Leptos 0.7 CSR** — reactive UI compiled to WASM via Trunk
- **web-sys** — localStorage, setTimeout, Blob/URL for CSV download, `fetch` for GitHub sync + library load
- **serde_json** — serializes state to localStorage and to GitHub

### Navigation
No router. A single `RwSignal<View>` in `AppState` drives which component renders. See `src/app.rs` (the `View` enum and `AppState::navigate()`). Bottom-nav and back-buttons set `view`; nothing else navigates.

### Global state (`src/app.rs`)
`AppState` is provided via Leptos context at the root and consumed everywhere with `expect_context::<AppState>()`. It holds:

- `goals: RwSignal<UserGoals>` — user training preferences, auto-saved on change
- `scheduled_workouts: RwSignal<Vec<ScheduledWorkout>>` — coach-generated or hand-imported workouts, keyed by `target_date`, auto-saved on change
- `history: RwSignal<Vec<ExerciseEntry>>` — finalized session records, auto-saved on change
- `active_session: RwSignal<Option<WorkoutSession>>` — the in-progress session, auto-saved (so a crash doesn't lose it)
- `session_drafts`, `custom_exercises`, `library` — supporting signals
- `view: RwSignal<View>` — current page
- `toast: RwSignal<Option<String>>` — 2.5s dismissing toast
- `sync_sha`, `last_synced_at`, `suppress_push` — GitHub sync coordination

`AppState` is `Copy` (all fields are `RwSignal<T>`, which are arena-backed IDs in Leptos 0.7).

### Boot + sync lifecycle
On `App` mount:
1. Library JSON is fetched asynchronously (non-blocking, just populates a signal).
2. If GitHub sync is configured, a boot pull runs. If `remote.updated_at > local_last_push_at` (or local is empty), the remote state hydrates into the signals. `boot_done` flips true regardless of pull outcome.
3. A debounced-push Effect watches every persisted signal. Once `boot_done` is true, any change schedules a 2-second debounce; on fire, the synced state is PUT to GitHub. A 409 conflict triggers a refetch + retry with the new sha. The 2-second debounce window matters — tests rely on it.

### Workout-session data flow
1. `HomeView` reads `scheduled_workouts` and finds today's. User taps *Start workout* → `app::new_session()` creates a `WorkoutSession` from the `ScheduledWorkout`.
2. `active_session` signal is set; nav moves to `SessionView`.
3. User logs sets via inputs + ✓ buttons; each interaction calls `active_session.update(|s| ...)`.
4. *Finish* → for each `ExerciseLog` with completed sets, an `ExerciseEntry { finalized: true }` is pushed into `history`. `active_session` is cleared.

### Key files

| File | Purpose |
|------|---------|
| `src/models.rs` | All data structs: `UserGoals`, `ScheduledWorkout`, `WorkoutSession`, `ExerciseLog`, `SetLog`, `ExerciseEntry`, legacy `WorkoutPlan` (kept for back-compat read only) |
| `src/storage.rs` | localStorage read/write helpers via `web_sys::Storage` |
| `src/sync.rs` | `SyncedState` (v2 schema), GitHub Contents API client, optimistic-concurrency push |
| `src/library.rs` | Exercise library loader and the `last_hit_by_muscle` / `recency_bucket` helpers used by the heatmap |
| `src/coach.rs` | Coach Brief packet generation — turns goals + history + library into the markdown an LLM sees |
| `src/csv_utils.rs` | History CSV export, file download helper |
| `src/app.rs` | `AppState`, `View` enum, `App` root, `BottomNav`, boot pull, debounced auto-push |
| `src/components/home.rs` | TODAY card + UPCOMING list + Recent Sessions + Coach Brief entry |
| `src/components/session.rs` | Session view: exercise accordions, set inputs, Finish |
| `src/components/exercises.rs` | Freeform Exercises tab + custom exercise creation |
| `src/components/library_view.rs` | Library browse + detail (silhouette uses `BodyMuscleHighlight`) |
| `src/components/history.rs` | History list + heatmap + per-session detail |
| `src/components/body_heatmap.rs` | `BodyHeatmap` (recency colors) + `BodyMuscleHighlight` (library detail) SVGs |
| `src/components/options.rs` | Goals editor + Sync settings + Coach Brief view |
| `scripts/coach/` | Out-of-app coach loop: `PROMPT.md`, `coach.sh`, `README.md` |

### Schema versions
`SyncedState.schema_version` is `2`. Legacy v1 (had a `plan` field, no `goals`/`scheduled_workouts`) still deserializes — `plan` is preserved on read but the app no longer uses it. When writing, the app emits v2.

### PWA / deployment
- `public/manifest.json` — Add-to-Home-Screen manifest
- `public/sw.js` — service worker. **Cache-first for app shell, network-first for `/data/*`** (exercise library), with HTTP-cache bypass on data fetches. Bumping the `CACHE` constant (e.g. `swolesome-v9` → `v10`) forces clients to drop the old cache and refetch the app shell on next load.
- `.github/workflows/deploy.yml` — builds via `trunk` and deploys `dist/` to GitHub Pages on push to `main`
- `.github/workflows/ci.yml` — Clippy + WASM unit tests + Playwright E2E

### CSV export
History export only (plan import/export removed in v2):
```
session_id,date,day_name,exercise_name,set_number,reps,weight,completed
```

## Heatmap muscle-name contract

`src/library.rs::last_hit_by_muscle` looks up an `ExerciseEntry`'s muscles by:
1. `exercise_id` matched against library entry IDs
2. Failing that, lowercase `exercise_name` matched against lowercase library names
3. Failing that, no muscles credited (entry is ignored for heatmap purposes)

So a freeform exercise named "Barbell Squat" will color the heatmap correctly because its name matches `Barbell Squat` in the library. A freeform exercise named "Squat With My Special Bar" will not, even if the user means the same lift. This is intentional — wrong colors are worse than uncolored.

Muscle keys used by both library and `body_heatmap.rs`: `abdominals`, `abductors`, `adductors`, `biceps`, `calves`, `chest`, `forearms`, `glutes`, `hamstrings`, `lats`, `lower back`, `middle back`, `neck`, `quadriceps`, `shoulders`, `traps`, `triceps`. Keep these in sync if the library source changes.

## Coach loop

Two entry points, same contract (defined in `scripts/coach/PROMPT.md`):

- **In-app:** Home → *Generate workout with Claude* → Coach Brief view. The packet rendered there is the same one the script would feed Claude.
- **Unattended:** `scripts/coach/coach.sh` pulls state via `gh`, fetches library, builds the brief, invokes `claude -p`, parses the JSON response, merges into `scheduled_workouts` (deduped by date), pushes back via the GitHub Contents API.

The intended deployment is to schedule `coach.sh` nightly via Cowork's `/schedule`. See `scripts/coach/README.md` for the invocation. Agents cannot create the Cowork routine for the user — the user has to run `/schedule` themselves.

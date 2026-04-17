# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A mobile-first PWA workout tracker built in Rust + Leptos (WASM), deployed to GitHub Pages. All state lives in `localStorage` — no backend. The target runtime is iPhone Safari.

## Commands

### First-time setup
```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

### Dev server (hot-reload)
```bash
trunk serve
# Opens at http://localhost:8080
```

### Production build
```bash
trunk build --release
# Output in ./dist/
```

### Build for GitHub Pages (sets correct base URL)
```bash
trunk build --release --public-url /wholesome-swolesome/
```

## Architecture

### Tech stack
- **Leptos 0.7 CSR** — reactive UI, compiled to WASM via Trunk
- **web-sys** — localStorage read/write, setTimeout, Blob/URL for CSV download
- **serde_json** — serializes the full plan and history to localStorage

### Navigation
No router. A single `RwSignal<View>` in `AppState` drives which component renders. See `src/app.rs` `View` enum and `AppState::navigate()`.

### Global state (`src/app.rs`)
`AppState` is provided via Leptos context at the root and consumed everywhere with `expect_context::<AppState>()`. It holds:
- `plan: RwSignal<WorkoutPlan>` — auto-saved to localStorage on every change
- `history: RwSignal<Vec<WorkoutSession>>` — auto-saved on every change
- `active_session: RwSignal<Option<WorkoutSession>>` — in-progress session, not persisted until "Finish"
- `view: RwSignal<View>` — current page
- `toast: RwSignal<Option<String>>` — 2.5s dismissing toast

`AppState` is `Copy` (all fields are `RwSignal<T>` which are arena-backed IDs in Leptos 0.7).

### Data flow for a workout session
1. `HomeView` → user taps a day → `app::new_session()` creates a `WorkoutSession` pre-filled with weights from the last session for that day
2. `active_session` signal is set; nav moves to `SessionView`
3. User logs sets via `+/−` steppers; each tap calls `active_session.update(|s| ...)`
4. "Finish" → marks `is_complete = true`, pushes into `history`, clears `active_session`

### Key files
| File | Purpose |
|------|---------|
| `src/models.rs` | All data structs: `WorkoutPlan`, `WorkoutDay`, `Exercise`, `WorkoutSession`, `ExerciseLog`, `SetLog` |
| `src/storage.rs` | `load_plan/save_plan/load_history/save_history` via `web_sys::Storage` |
| `src/seed.rs` | Default 7-day plan (seeded from `workout_plan.csv`) |
| `src/csv_utils.rs` | `export_plan_csv`, `import_plan_csv`, `export_history_csv`, `download_file` |
| `src/app.rs` | `AppState`, `View` enum, `App` root component, `BottomNav`, `new_session()` |
| `src/components/` | One file per screen: `home`, `session`, `history`, `plan_editor`, `progress` |

### PWA / deployment
- `public/manifest.json` — web app manifest for "Add to Home Screen"
- `public/sw.js` — cache-first service worker for offline use
- `.github/workflows/deploy.yml` — builds with `trunk` and deploys `dist/` to GitHub Pages on push to `main`

### CSV formats

**Plan export/import** (`workout_plan.csv`):
```
day_id,day_name,exercise_id,exercise_name,target_sets,reps_min,reps_max,category,notes
```

**History export** (`workout_history.csv`):
```
session_id,date,day_name,exercise_name,set_number,reps,weight_lbs,completed
```

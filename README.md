# Wholesome Swolesome 💪

A mobile-first PWA workout tracker built in Rust + Leptos (WASM). Track sets, reps, and weight while you're in the gym. No account, no backend — all data lives in your browser's localStorage.

**Live app:** https://tbaums.github.io/wholesome-swolesome/

## Features

- Log sets, reps, and weight for each exercise
- Weights auto-fill from your last session for that day
- Session progress saved automatically — survives closing the browser
- Full workout history with per-session detail view
- Progress charts per exercise
- Editable workout plan with CSV import/export
- Installable as a PWA (Add to Home Screen on iPhone)

## Tech stack

- [Leptos](https://leptos.dev) 0.7 (CSR) compiled to WASM via [Trunk](https://trunkrs.dev)
- `web-sys` for localStorage, Blob/URL for CSV download
- `serde_json` for serialization
- Deployed to GitHub Pages via GitHub Actions

## Development

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve          # dev server at http://localhost:8080
trunk build --release --public-url /wholesome-swolesome/  # production build
```

## Importing a workout plan

Go to **Plan → Import / Export → Import Plan** and paste a CSV with this header:

```
day_id,day_name,exercise_id,exercise_name,target_sets,reps_min,reps_max,category,notes
```

**Column reference**

| Column | Required | Notes |
|--------|----------|-------|
| `day_id` | Yes | Arbitrary string that groups exercises into the same day. Rows with the same `day_id` become one day. |
| `day_name` | Yes | Display name for the day, e.g. `Lower A`. |
| `exercise_id` | No | Leave blank to auto-generate a UUID. Supply one if you want stable IDs across re-imports. |
| `exercise_name` | Yes | Display name for the exercise. |
| `target_sets` | Yes | Integer — how many sets to pre-fill. |
| `reps_min` | Yes | Integer — lower end of the rep range. |
| `reps_max` | Yes | Integer — upper end of the rep range. |
| `category` | Yes | `Main`, `Core`, or `Cardio` (case-insensitive). Anything else is treated as `Main`. |
| `notes` | No | Free text. Can be empty or omitted. |

**Example**

```csv
day_id,day_name,exercise_id,exercise_name,target_sets,reps_min,reps_max,category,notes
d1,Lower A,,Hip Thrust,4,6,10,Main,
d1,Lower A,,Romanian Deadlift,3,8,12,Main,
d1,Lower A,,Leg Press,3,10,15,Main,
d2,Upper A,,Bench Press,4,5,8,Main,
d2,Upper A,,Pull-Up,3,6,10,Main,
d3,Core,,Plank,3,30,60,Core,seconds not reps
```

- The header row is required.
- Empty rows are ignored.
- Importing replaces your entire current plan — export first if you want a backup.
- You can download your existing plan as a starting point via **Download Plan CSV**.

## Data model

### Storage keys (localStorage)

| Key | Type |
|-----|------|
| `ws_plan` | `WorkoutPlan` |
| `ws_ex_history` | `Vec<ExerciseEntry>` |
| `ws_active_session` | `Option<WorkoutSession>` |
| `ws_session_drafts` | `Vec<WorkoutSession>` |
| `ws_custom_exercises` | `Vec<Exercise>` |

### Structs

**`WorkoutPlan`** — the user's program
Contains a list of `WorkoutDay`s. Each day has a name ("Lower A", "Push", etc.) and a list of `Exercise`s. This is what drives the Workout tab's day selector and the Exercises tab's card list.

**`Exercise`** — a movement definition
`id`, `name`, `target_sets`, `reps_min`, `reps_max`, `category` (Main/Core/Cardio), optional `notes`. These are templates — they define what to do, not what was done.

**`WorkoutSession`** — an in-progress or completed named workout
Lives in `active_session` while ongoing (persisted so a crash doesn't lose it). Has a `day_id`/`day_name`, a `date`, and a list of `ExerciseLog`s.

**`ExerciseLog`** — one exercise within a session
Mirrors the plan's `Exercise` metadata (name, target sets, rep range) plus a list of `SetLog`s that get filled in as the user works out.

**`SetLog`** — one set
`set_number`, `reps`, `weight`, `completed`, `completed_date` (the YYYY-MM-DD the ✓ was tapped — set at click time, not session-finish time).

**`ExerciseEntry`** — a finalized history record
This is the flat history format. On "Finish Workout", `WorkoutSession` is converted: one `ExerciseEntry` per exercise *per completion date* (sets done across midnight land on the correct day). Also used by the Exercises tab for freeform logging — a non-`finalized` entry is the "active" in-progress record for that exercise today.

Key fields: `id`, `date`, `exercise_name`, `exercise_id`, `session_id` (links back to the originating session, `None` for freeform), `day_id`/`day_name` (`None` for freeform), `sets` (only completed sets after finalization), `finalized`, `created_at` (ISO 8601 timestamp for sort order).

**Custom exercises** (`Vec<Exercise>` in `ws_custom_exercises`)
Exercises created directly in the Exercises tab. Structurally identical to plan exercises but stored separately so they don't appear as a workout day.

### Relationships

```
WorkoutPlan
  └── WorkoutDay[]
        └── Exercise[]          ← templates

active_session: WorkoutSession
  └── ExerciseLog[]             ← mirrors Exercise metadata + live SetLogs
        └── SetLog[]            ← completed_date stamped at ✓ click

history: ExerciseEntry[]        ← permanent record after finish
  └── SetLog[]                  ← only completed sets kept

custom_exercises: Exercise[]    ← freeform-only, never in a WorkoutDay
```

The Exercises tab uses `ExerciseEntry` with `finalized: false` as a scratchpad for today's freeform work. When the ✓ button is hit, it flips to `finalized: true` (keeping only checked sets) and becomes a permanent history record — same shape as session-derived entries.

## License

MIT — see [LICENSE](LICENSE).

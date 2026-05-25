# Plan: Stretching & Balance Exercises in Daily Workouts

## Problem

The app currently has only 2 stretching exercises (Cat Stretch, Child's Pose) and 0 balance exercises in its library. The coach prompt mentions "mobility" as an afterthought but has no structured rules for prescribing stretching or balance work. These modalities should be part of every daily workout.

## Key Design Decision: Duration-Based Exercises

Stretching and balance exercises are fundamentally different from strength work — they use **timed holds** (e.g., 30s), not weight × reps. The current `SetLog` only tracks `reps: u32` and `weight: f32`. We need to extend the data model to support duration-based sets.

**Approach:** Add an optional `duration_seconds` field alongside existing reps/weight fields. This is backwards-compatible (existing data deserializes fine with `serde(default)`), works for both timed holds and rep-based balance drills, and avoids splitting the session UI into two completely different modes.

---

## Changes by File

### 1. Exercise Library — `public/data/exercises.json`

Add **~25 stretching** and **~12 balance** exercises covering all major muscle groups. Each entry follows the existing `LibraryExercise` schema.

**Stretching exercises to add** (category: `"stretching"`, force: `"static"`, equipment: mostly `null` or `"body only"`):

| Name | Primary Muscles | Secondary Muscles |
|------|----------------|-------------------|
| Standing Hamstring Stretch | hamstrings | lower back, calves |
| Seated Forward Bend | hamstrings | lower back |
| Standing Quad Stretch | quadriceps | — |
| Pigeon Pose | glutes | abductors |
| Figure Four Stretch | glutes | abductors |
| Hip Flexor Lunge Stretch | quadriceps | abdominals, glutes |
| Butterfly Stretch | adductors | glutes |
| Standing Calf Stretch | calves | — |
| Doorway Chest Stretch | chest | shoulders |
| Cross-Body Shoulder Stretch | shoulders | middle back |
| Overhead Triceps Stretch | triceps | shoulders |
| Standing Biceps Wall Stretch | biceps | chest |
| Lat Side Stretch | lats | abdominals |
| Seated Spinal Twist | lower back | abdominals, glutes |
| Neck Side Bend Stretch | neck | traps |
| Neck Forward Bend Stretch | neck | traps |
| Standing Side Bend | abdominals | lats |
| Lying Glute Stretch | glutes | lower back |
| World's Greatest Stretch | quadriceps | hamstrings, glutes, chest, shoulders |
| Supine Spinal Twist | lower back | glutes, abdominals |
| Wall Lat Stretch | lats | shoulders |
| Frog Stretch | adductors | glutes |
| Scorpion Stretch | chest | shoulders, abdominals |
| Seated Trap Stretch | traps | neck |
| Forearm Wall Stretch | forearms | biceps |

**Balance exercises to add** (category: `"balance"`, equipment: mostly `"body only"` or `"exercise ball"`):

| Name | Primary Muscles | Secondary Muscles |
|------|----------------|-------------------|
| Single-Leg Stand | calves | glutes, quadriceps |
| Single-Leg Deadlift (Bodyweight) | hamstrings | glutes, lower back |
| Bosu Ball Squat | quadriceps | glutes, calves |
| Single-Leg Calf Raise (Balance) | calves | glutes |
| Tandem Stance Hold | calves | abdominals |
| Single-Leg Hip Hinge | hamstrings | glutes, lower back |
| Stability Ball Plank | abdominals | shoulders, lower back |
| Bird Dog | lower back | glutes, abdominals |
| Side Plank with Leg Lift | abdominals | abductors, glutes |
| Single-Leg Glute Bridge | glutes | hamstrings, lower back |
| Pallof Press Hold | abdominals | shoulders |
| BOSU Ball Single-Leg Stand | calves | glutes, quadriceps |

Note: `"balance"` is a new category value. The `LibraryExercise.category` field is a `String`, not an enum, so no Rust-side schema change is needed.

### 2. Data Model — `src/models.rs`

**SetLog** — add optional duration field:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SetLog {
    pub set_number: u32,
    pub reps: u32,
    #[serde(alias = "weight_lbs")]
    pub weight: f32,
    pub completed: bool,
    #[serde(default)]
    pub completed_date: Option<String>,
    #[serde(default)]
    pub duration_seconds: Option<u32>,  // NEW — for stretching/balance holds
}
```

**ScheduledExercise** — add optional duration prescription:
```rust
pub struct ScheduledExercise {
    pub library_id: Option<String>,
    pub name: String,
    pub target_sets: u32,
    pub reps_min: u32,
    pub reps_max: u32,
    #[serde(default)]
    pub rest_seconds: u32,
    pub notes: Option<String>,
    #[serde(default)]
    pub target_duration_seconds: Option<u32>,  // NEW — e.g., 30 for a 30s hold
}
```

**ExerciseLog** — mirror the duration prescription:
```rust
pub struct ExerciseLog {
    pub exercise_id: String,
    pub exercise_name: String,
    pub target_sets: u32,
    pub reps_min: u32,
    pub reps_max: u32,
    pub sets: Vec<SetLog>,
    #[serde(default)]
    pub target_duration_seconds: Option<u32>,  // NEW
}
```

**ExerciseEntry** — same addition for history:
```rust
pub struct ExerciseEntry {
    // ... existing fields ...
    #[serde(default)]
    pub target_duration_seconds: Option<u32>,  // NEW
}
```

All new fields use `serde(default)` so existing persisted data and synced `state.json` files deserialize without error (they'll be `None` / `0`).

### 3. Session UI — `src/components/session.rs`

Modify the set-row rendering to detect duration-based exercises and show a different input:

- **If `target_duration_seconds.is_some()`**: Show a **duration input** (seconds) instead of weight+reps. Label: "Hold for Xs". The done checkbox stays the same.
- **If `target_duration_seconds.is_none()`**: Existing weight + reps UI unchanged.

Concretely, in the `SetRow` component:
- When duration mode: render one `<input type="number" step=5 placeholder="sec">` bound to `set.duration_seconds`.
- The `toggle_done` logic is identical — just marks the set complete.

Display format in the exercise accordion header:
- Strength: "4 × 6-8"
- Duration: "3 × 30s" (using `target_duration_seconds`)

### 4. Home View — `src/components/home.rs`

Update the exercise list rendering in `ScheduledCard`:
- **Duration exercises**: Show `"{target_sets} × {target_duration_seconds}s"` (e.g., "3 × 30s")
- **Rep exercises**: Keep existing `"{target_sets}×{reps_min}-{reps_max}"` format

### 5. Session Creation — `src/app.rs`

In `new_session_from_scheduled()`:
- Pass `target_duration_seconds` through from `ScheduledExercise` to `ExerciseLog`.
- When pre-filling sets for a duration exercise, set `duration_seconds` to the target value (instead of looking up last weight/reps).

In `finish()` (session finalization):
- Pass `target_duration_seconds` through to `ExerciseEntry`.

### 6. Coach Prompt — `scripts/coach/PROMPT.md`

Add a new section after "Movement balance" in the Step 4 rules:

**Stretching & Balance Protocol:**

> **Stretching (every session)**:
> - Include 3-5 stretching exercises at the end of every workout as a cooldown block.
> - Target muscles worked during that session, plus any muscles in the "Stale" recovery bucket.
> - Prescribe `target_sets: 2`, `reps_min: 1`, `reps_max: 1`, `target_duration_seconds: 30` (a 30-second hold, 2 sets).
> - `rest_seconds: 10` (just enough to switch sides).
> - For dynamic/warm-up stretches at the start, use `reps_min/reps_max` (e.g., 10-15 reps) with no `target_duration_seconds`.
>
> **Balance (2-3 sessions per week)**:
> - Include 2-3 balance exercises per session, placed after main strength work and before stretching.
> - Alternate between lower-body balance (single-leg stands, single-leg deadlifts) and core stability (bird dog, stability ball plank).
> - For timed holds: `target_sets: 2-3`, `reps_min: 1`, `reps_max: 1`, `target_duration_seconds: 20-45`.
> - For rep-based balance drills: use normal `reps_min`/`reps_max` (e.g., 8-12 per side), no `target_duration_seconds`.

Update the JSON schema example in Step 5 to show a stretching exercise:
```json
{
  "library_id": "Standing_Hamstring_Stretch",
  "name": "Standing Hamstring Stretch",
  "target_sets": 2,
  "reps_min": 1,
  "reps_max": 1,
  "target_duration_seconds": 30,
  "rest_seconds": 10,
  "notes": "Hold each side 30s; breathe through the stretch"
}
```

Update the exercise ordering rule:
> **Order**: Compounds before isolations. Heaviest/most-demanding first. Balance work after main lifts. Stretching last (cooldown). Any cardio before or after balance, depending on goal.

Update the session time budget rule to account for stretching/balance:
> Reserve ~5-8 minutes for the cooldown stretch block and ~5-7 minutes for balance work when included. Subtract this from `session_minutes` before budgeting strength sets.

### 7. In-App Coach Brief — `src/coach.rs`

Update the library listing format to include category so the coach can distinguish stretching/balance exercises from strength:
- Already included: the pipe-delimited listing has `category`. No change needed here.

Update the "Task" instructions at the bottom of the coach packet to reference the new stretching/balance rules (mirror the PROMPT.md additions).

### 8. CSV Export — `src/csv_utils.rs`

Add `duration_seconds` as an optional column in the CSV export:
```
session_id,date,day_name,exercise_name,set_number,reps,weight,duration_seconds,completed
```

Use empty string when `duration_seconds` is `None` to maintain backwards compatibility with existing CSV consumers.

### 9. Sync Schema — `src/sync.rs`

No `schema_version` bump needed. All new fields are additive with `serde(default)`, so v2 still reads/writes correctly. Old clients ignore unknown fields; new clients default missing fields to `None`.

### 10. CLAUDE.md

Add a note about the duration-based exercise convention and the new `"balance"` category to the existing documentation sections.

---

## What Does NOT Change

- **Heatmap / muscle taxonomy**: Stretching and balance exercises use the same 17 muscle keys. `last_hit_by_muscle` will credit them automatically via the existing library lookup. No heatmap code changes.
- **Recovery buckets**: Stretching doesn't count as "high-intensity work" per the coach rules — the coach already allows "light accessory / mobility" for muscles in the ≤3 day bucket. No bucket logic changes.
- **Library validation**: `validate_exercises_against_library` checks `library_id` against the library. New exercises just need valid IDs. No validation code changes.
- **Sync protocol**: Additive fields, no version bump.

---

## Implementation Order

1. **Models** (`src/models.rs`) — add duration fields
2. **Exercise library** (`public/data/exercises.json`) — add ~37 exercises
3. **Session UI** (`src/components/session.rs`) — duration input mode
4. **Home view** (`src/components/home.rs`) — duration display format
5. **Session creation** (`src/app.rs`) — pass duration through
6. **Coach prompt** (`scripts/coach/PROMPT.md`) — stretching/balance rules
7. **In-app coach** (`src/coach.rs`) — mirror prompt updates
8. **CSV export** (`src/csv_utils.rs`) — add column
9. **CLAUDE.md** — document conventions

Steps 1-2 can be done in parallel. Steps 3-5 depend on 1. Steps 6-7 are independent of 3-5. Step 8 depends on 1.

---

## Testing

- **Unit tests**: Verify `SetLog` / `ScheduledExercise` deserialization with and without the new duration fields (backwards compat).
- **Playwright E2E**: Start a session with a scheduled workout containing a stretching exercise, verify the duration input renders, complete the set, finish the session, check history.
- **Manual**: Generate a coach workout via the brief, verify stretching/balance exercises appear at the end, verify the session UI works on iPhone Safari.

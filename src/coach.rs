//! Generates the "coach packet" markdown that gets fed to Claude, and parses
//! the JSON workout it returns.

use std::collections::{HashMap, HashSet};

use crate::library::{category_of, days_between, last_hit_by_muscle, recency_bucket, RecencyBucket};
use crate::models::{
    ExerciseEntry, FocusLevel, LibraryExercise, ScheduledExercise, ScheduledWorkout, UserGoals,
    WorkoutSource, ALL_MUSCLES,
};

const HISTORY_WINDOW_DAYS: i64 = 14;
const RECENT_SCHEDULED_DAYS: i64 = 7;
const CARDIO_WINDOW_DAYS: i64 = 7;

/// Optional vitals block returned alongside a workout by the off-app coach.
/// Currently just VO2 max + the date it was read on (so the importer can drop
/// stale screenshots silently).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Vitals {
    pub vo2_max: f32,
    pub source_date: String,
}

/// Result of parsing the coach's JSON response.
#[derive(Debug)]
pub struct ParsedResponse {
    pub workout: ScheduledWorkout,
    pub vitals: Option<Vitals>,
}

pub struct PacketInput<'a> {
    pub goals: &'a UserGoals,
    pub history: &'a [ExerciseEntry],
    pub library: &'a [LibraryExercise],
    pub scheduled: &'a [ScheduledWorkout],
    pub today: &'a str,
    pub target_date: &'a str,
}

pub fn build_coach_packet(input: PacketInput<'_>) -> String {
    let mut out = String::new();

    out.push_str("# Wholesome Swolesome — Coach Brief\n\n");
    out.push_str(&format!(
        "**Plan a workout for:** {} (today is {})\n\n",
        input.target_date, input.today
    ));

    // ── Goals ────────────────────────────────────────────────────────────────
    out.push_str("## Goals\n");
    out.push_str(&format!("- Primary goal: **{}**\n", input.goals.primary_goal.label()));
    out.push_str(&format!("- Sessions per week: {}\n", input.goals.sessions_per_week));
    out.push_str(&format!("- Target session length: {} min\n", input.goals.session_minutes));
    if !input.goals.equipment.is_empty() {
        out.push_str(&format!(
            "- Available equipment: {}\n",
            input.goals.equipment.join(", ")
        ));
    } else {
        out.push_str("- Available equipment: assume full commercial gym\n");
    }
    if !input.goals.avoid.trim().is_empty() {
        out.push_str(&format!("- Avoid / injuries: {}\n", input.goals.avoid.trim()));
    }
    if !input.goals.notes.trim().is_empty() {
        out.push_str(&format!("- Notes: {}\n", input.goals.notes.trim()));
    }
    out.push('\n');

    // ── Cardio & mobility targets ──────────────────────────────────────────
    out.push_str("## Cardio & mobility targets\n");
    let cardio_done = cardio_minutes_in_window(
        input.history,
        input.library,
        input.today,
        CARDIO_WINDOW_DAYS,
    );
    if let Some(target) = input.goals.weekly_cardio_minutes_target {
        out.push_str(&format!(
            "- Weekly cardio minutes target: **{target}** (logged in last {CARDIO_WINDOW_DAYS}d: {cardio_done})\n"
        ));
    } else {
        out.push_str(&format!(
            "- Weekly cardio minutes target: _none set_ (logged in last {CARDIO_WINDOW_DAYS}d: {cardio_done})\n"
        ));
    }
    match (input.goals.vo2_max_latest, input.goals.vo2_max_updated.as_deref()) {
        (Some(v), Some(d)) => out.push_str(&format!("- VO2 max: **{v:.1}** (updated {d})\n")),
        (Some(v), None) => out.push_str(&format!("- VO2 max: **{v:.1}**\n")),
        _ => out.push_str("- VO2 max: _not provided_\n"),
    }
    out.push_str(&format!(
        "- Mobility focus: **{}** · Balance focus: **{}**\n",
        input.goals.mobility_focus.label(),
        input.goals.balance_focus.label(),
    ));
    out.push_str(&format!(
        "{}\n",
        mobility_balance_hint(input.goals.mobility_focus, input.goals.balance_focus),
    ));
    out.push('\n');

    // ── Mobility recovery (days since each muscle was last stretched) ──────
    out.push_str("## Mobility recovery (days since last stretched)\n\n");
    let stretched = last_stretched_by_muscle(input.history, input.library);
    out.push_str("| Muscle | Days since stretched |\n|---|---:|\n");
    for m in ALL_MUSCLES {
        let cell = match stretched.get(*m) {
            Some(date) => days_between(date, input.today).unwrap_or(999).to_string(),
            None => "∞".into(),
        };
        out.push_str(&format!("| {m} | {cell} |\n"));
    }
    out.push('\n');

    // ── Screenshot-attach hint ─────────────────────────────────────────────
    out.push_str(
        "> **Tip:** If you have an Apple Health VO2 max screenshot, paste it into this conversation alongside the brief. Read the value + the date it covers from the screenshot, and include them in a top-level `vitals` block of your JSON response (see Response format below).\n\n"
    );

    // ── Muscle recovery state ───────────────────────────────────────────────
    out.push_str("## Muscle recovery state (days since last worked)\n\n");
    let last = last_hit_by_muscle(input.history, input.library);
    out.push_str("| Muscle | Days ago | Recovery |\n|---|---:|---|\n");
    for m in ALL_MUSCLES {
        let (days_str, recovery) = match last.get(*m) {
            Some(date) => {
                let d = days_between(date, input.today).unwrap_or(999);
                let bucket = recency_bucket(d).unwrap_or(RecencyBucket::Stale);
                (format!("{d}"), bucket.label().to_string())
            }
            None => ("∞".into(), "never".into()),
        };
        out.push_str(&format!("| {m} | {days_str} | {recovery} |\n"));
    }
    out.push('\n');

    // ── Recent history (last 14 days) ───────────────────────────────────────
    out.push_str(&format!(
        "## Recent training (last {HISTORY_WINDOW_DAYS} days)\n\n"
    ));
    let recent = recent_history(input.history, input.today, HISTORY_WINDOW_DAYS);
    if recent.is_empty() {
        out.push_str("_No completed sessions in the window._\n\n");
    } else {
        let by_date = group_by_date(&recent);
        let mut dates: Vec<_> = by_date.keys().collect();
        dates.sort();
        dates.reverse();
        for date in dates {
            let entries = &by_date[date];
            let day_name = entries
                .iter()
                .find_map(|e| e.day_name.as_deref())
                .unwrap_or("Freeform");
            out.push_str(&format!("### {date} — {day_name}\n"));
            for e in entries {
                let done: Vec<_> = e.sets.iter().filter(|s| s.completed).collect();
                let summary: Vec<String> = done
                    .iter()
                    .map(|s| {
                        if let Some(dur) = s.duration_seconds {
                            format!("{}s", dur)
                        } else if s.weight > 0.0 {
                            format!("{}x{:.0}", s.reps, s.weight)
                        } else {
                            format!("{}", s.reps)
                        }
                    })
                    .collect();
                out.push_str(&format!(
                    "- **{}** — {} sets: {}\n",
                    e.exercise_name,
                    done.len(),
                    if summary.is_empty() {
                        "—".into()
                    } else {
                        summary.join(", ")
                    }
                ));
            }
            out.push('\n');
        }
    }

    // ── Already scheduled (avoid duplicates) ────────────────────────────────
    let upcoming = recent_scheduled(input.scheduled, input.today, RECENT_SCHEDULED_DAYS);
    if !upcoming.is_empty() {
        out.push_str(&format!(
            "## Already scheduled (next {RECENT_SCHEDULED_DAYS} days)\n\n"
        ));
        for w in &upcoming {
            out.push_str(&format!("- **{}** on {}: {} exercises\n", w.name, w.date, w.exercises.len()));
        }
        out.push('\n');
    }

    // ── Library listing (inline so the off-app Claude knows what's available) ─
    out.push_str("## Exercise library — the ONLY valid sources of exercises\n\n");
    out.push_str(&format!(
        "The app stores every exercise as an `id` from this list. Every exercise in your response **MUST** use a `library_id` taken verbatim from the `id` column below — the importer rejects anything else. Do not invent ids, do not lowercase or rewrite them. {} entries:\n\n",
        input.library.len()
    ));
    out.push_str(&render_library_listing(input.library));
    out.push('\n');

    // ── Instructions ────────────────────────────────────────────────────────
    out.push_str(
        r#"## Task

Design ONE workout for the target date. Apply:
- **Library-only** — every exercise must have a `library_id` copied verbatim from the table above. If the lift you want isn't in the table, pick the closest entry that IS rather than inventing one. No freeform names.
- **Progressive overload** — if the user hit the top of their rep range on an exercise recently with good completion, bump weight ~2.5% (or one notch). If they missed reps, hold weight.
- **Recovery science** — avoid muscles worked in the last 48h for high-intensity work; touch them only with low-volume accessory work if at all. Prioritize muscles in the "8-14 days" / "never" buckets.
- **Movement balance** — across a week, balance push/pull, knee-dominant / hip-dominant, vertical / horizontal.
- **Volume** — match `session_minutes`. Plan ~10-25 working sets total. Include rest_seconds appropriate for the goal (60-90s for hypertrophy, 180-300s for strength).
- **Equipment** — only prescribe exercises whose `equipment` is in the user's available list (empty list = full commercial gym, all equipment OK).
- **Stretching** — see the user's mobility focus above. Default (Standard) is 3-5 stretches as a cooldown each session, prioritizing muscles with high "days since last stretched" in the table above. Use `target_sets: 2`, `reps_min: 1`, `reps_max: 1`, `target_duration_seconds: 30`, `rest_seconds: 10`.
- **Balance** — see the user's balance focus above. Default (Standard) is 2-3 balance exercises after main lifts on 2-3 sessions per week. For timed holds: `target_sets: 2-3`, `reps_min: 1`, `reps_max: 1`, `target_duration_seconds: 20-45`. For rep-based balance drills: use normal reps, omit `target_duration_seconds`.
- **Cardio** — if a weekly cardio minutes target is set above and the logged-in-last-7d number is short, include a cardio piece (category=cardio) sized to close the gap. Two encodings:
  - **Preferred: HR-zone breakdown.** Add a `target_zones` array of `{zone, minutes}` objects using Apple-Watch zones 1-5 (Z1 very light, Z2 easy aerobic, Z3 moderate, Z4 threshold, Z5 max). For zone 2 base work: `[{zone: 2, minutes: 30}]`. For intervals like "5 min warm-up Z1, 4×4 min Z4 with 3 min Z1 between, 5 min cool-down Z1": `[{zone: 1, minutes: 13}, {zone: 4, minutes: 16}]` (sum the per-zone time across the session). Set `reps_min` and `reps_max` to the **total** minutes (sum of all zone minutes) so the existing minutes target stays accurate, and `target_sets: 1`. Omit `weight`/RPE — the app shows per-zone inputs.
  - **Fallback: total minutes + RPE.** If a zone breakdown isn't clearly warranted (e.g. casual "walk 20 min"), omit `target_zones`. Then `reps` is minutes and `weight` is RPE 1-10; `target_sets: 1`.
  - Apply progressive overload either way — bump minutes ~5-10% or one zone notch up if the prior session was completed cleanly.
- **Session time budget** — reserve ~5 min for cooldown stretches and ~5 min for balance work. If cardio is included, budget its minutes explicitly. Subtract all of these from `session_minutes` before budgeting strength sets.
- **Order** — compounds before isolations, heaviest first. Balance work after main lifts. Cardio either as a warm-up (zone 2, before strength) or finisher (after strength, before cooldown). Stretching last (cooldown).

## Response format

Wrap your reply in a single fenced ```json code block — nothing else, no commentary before or after. The fence is important: it makes the response render as a code card in the Claude UI with a one-tap copy button, so the user can copy the whole JSON straight into the app's paste box. The app's importer strips the fence when parsing.

```json
{
  "name": "Short workout title (e.g. 'Upper Push + Posterior Chain')",
  "rationale": "1-3 sentence explanation: which muscles, why this volume, recovery considerations.",
  "exercises": [
    {
      "library_id": "Barbell_Bench_Press_-_Medium_Grip",
      "name": "Bench Press",
      "target_sets": 4,
      "reps_min": 6,
      "reps_max": 8,
      "rest_seconds": 180,
      "notes": "RPE 7-8; pause 1s at bottom"
    },
    {
      "library_id": "Standing_Hamstring_Stretch",
      "name": "Standing Hamstring Stretch",
      "target_sets": 2,
      "reps_min": 1,
      "reps_max": 1,
      "target_duration_seconds": 30,
      "rest_seconds": 10,
      "notes": "Hold each side 30s"
    },
    {
      "library_id": "Running_Treadmill",
      "name": "Running, Treadmill",
      "target_sets": 1,
      "reps_min": 29,
      "reps_max": 29,
      "target_zones": [
        {"zone": 1, "minutes": 13},
        {"zone": 4, "minutes": 16}
      ],
      "rest_seconds": 0,
      "notes": "5 min Z1 warm-up · 4×4 min Z4 with 3 min Z1 between · 5 min Z1 cool-down"
    }
  ]
}
```

`library_id` is **required** for every exercise and must exactly match an `id` from the table above. `name` should match the library `name` for consistency. `notes` can be null. For stretching/balance exercises with timed holds, include `target_duration_seconds` — the app shows a seconds input instead of weight × reps. For cardio with a clear zone breakdown, include `target_zones` — the app shows one input per zone instead of min × RPE; otherwise omit it and the app falls back to `reps`=minutes + `weight`=RPE. Order exercises in the sequence they should be performed.

### Optional vitals block

If you extracted a VO2 max reading from an Apple Health screenshot pasted into this conversation, add a top-level `vitals` block to the JSON:

```json
{
  "name": "...",
  "exercises": [ ... ],
  "vitals": {
    "vo2_max": 36.4,
    "source_date": "2026-05-27"
  }
}
```

`source_date` is the date the reading is from (YYYY-MM-DD). If the screenshot doesn't include vitals, omit the block entirely. The importer drops vitals whose `source_date` is older than what the user already has.
"#,
    );

    out
}

fn mobility_balance_hint(mobility: FocusLevel, balance: FocusLevel) -> &'static str {
    match (mobility, balance) {
        (FocusLevel::High, FocusLevel::High) => "  _High focus on both → 5-7 stretches and 3+ balance drills per session._",
        (FocusLevel::High, _) => "  _High mobility focus → bump cooldown stretches to 5-7._",
        (_, FocusLevel::High) => "  _High balance focus → include 3+ balance drills every session._",
        (FocusLevel::Low, FocusLevel::Low) => "  _Low focus on both → 0-1 stretches OK, balance only when explicitly stale._",
        (FocusLevel::Low, _) => "  _Low mobility focus → 1-2 stretches max per session._",
        (_, FocusLevel::Low) => "  _Low balance focus → skip balance unless a critical muscle group is stale._",
        _ => "  _Standard focus → default cadence (3-5 stretches; balance 2-3 sessions/wk)._",
    }
}

/// Compact pipe-delimited listing the off-app Claude can scan to pick valid IDs.
/// Columns: id | name | equipment | category | primary | secondary.
pub fn render_library_listing(library: &[LibraryExercise]) -> String {
    let mut sorted: Vec<&LibraryExercise> = library.iter().collect();
    sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let mut out = String::new();
    out.push_str("```\n");
    out.push_str("id | name | equipment | category | primary | secondary\n");
    for ex in sorted {
        let equipment = ex.equipment.as_deref().unwrap_or("-");
        let primary = if ex.primary_muscles.is_empty() {
            "-".to_string()
        } else {
            ex.primary_muscles.join(",")
        };
        let secondary = if ex.secondary_muscles.is_empty() {
            "-".to_string()
        } else {
            ex.secondary_muscles.join(",")
        };
        out.push_str(&format!(
            "{} | {} | {} | {} | {} | {}\n",
            ex.id, ex.name, equipment, ex.category, primary, secondary
        ));
    }
    out.push_str("```\n");
    out
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn recent_history<'a>(history: &'a [ExerciseEntry], today: &str, window: i64) -> Vec<&'a ExerciseEntry> {
    history
        .iter()
        .filter(|e| {
            e.finalized
                && e.sets.iter().any(|s| s.completed)
                && (0..=window).contains(&days_between(&e.date, today).unwrap_or(9999))
        })
        .collect()
}

fn group_by_date<'a>(entries: &'a [&'a ExerciseEntry]) -> HashMap<&'a str, Vec<&'a ExerciseEntry>> {
    let mut out: HashMap<&'a str, Vec<&'a ExerciseEntry>> = HashMap::new();
    for e in entries {
        out.entry(e.date.as_str()).or_default().push(e);
    }
    out
}

fn recent_scheduled<'a>(
    scheduled: &'a [ScheduledWorkout],
    today: &str,
    window: i64,
) -> Vec<&'a ScheduledWorkout> {
    let mut v: Vec<&'a ScheduledWorkout> = scheduled
        .iter()
        .filter(|w| {
            let d = days_between(today, &w.date).unwrap_or(-9999);
            (0..=window).contains(&d)
        })
        .collect();
    v.sort_by(|a, b| a.date.cmp(&b.date));
    v
}

/// Sum of minutes (= `reps` for cardio exercises) across completed cardio sets
/// in the last `window` days.
pub fn cardio_minutes_in_window(
    history: &[ExerciseEntry],
    library: &[LibraryExercise],
    today: &str,
    window: i64,
) -> u32 {
    let mut total: u32 = 0;
    for e in history {
        if !(0..=window).contains(&days_between(&e.date, today).unwrap_or(9999)) {
            continue;
        }
        let cat = category_of(&e.exercise_id, &e.exercise_name, library);
        if cat != Some("cardio") {
            continue;
        }
        for s in &e.sets {
            if !s.completed { continue; }
            // Zone-shaped cardio: sum per-zone actual minutes (f32, can be fractional
            // since Apple Health samples HR continuously). Round per-set into the
            // weekly u32 total so a string of decimals doesn't accumulate truncation
            // bias.
            if let Some(zones) = &s.zone_minutes {
                let set_total: f32 = zones.iter().map(|z| z.minutes).sum();
                total = total.saturating_add(set_total.round().max(0.0) as u32);
            } else {
                // Legacy: `reps` stores minutes directly.
                total = total.saturating_add(s.reps);
            }
        }
    }
    total
}

/// Most-recent date each muscle was hit by a *stretching* exercise.
/// Used so the coach can prioritize stretches for under-mobilized muscles.
pub fn last_stretched_by_muscle(
    history: &[ExerciseEntry],
    library: &[LibraryExercise],
) -> HashMap<String, String> {
    let by_id: HashMap<&str, &LibraryExercise> =
        library.iter().map(|e| (e.id.as_str(), e)).collect();
    let by_name: HashMap<String, &LibraryExercise> = library
        .iter()
        .map(|e| (e.name.to_lowercase(), e))
        .collect();

    let mut last: HashMap<String, String> = HashMap::new();
    for entry in history {
        if !entry.sets.iter().any(|s| s.completed) {
            continue;
        }
        let lib_entry = by_id
            .get(entry.exercise_id.as_str())
            .copied()
            .or_else(|| by_name.get(&entry.exercise_name.to_lowercase()).copied());
        let Some(lib_entry) = lib_entry else { continue };
        if lib_entry.category != "stretching" {
            continue;
        }
        for m in lib_entry.primary_muscles.iter().chain(lib_entry.secondary_muscles.iter()) {
            last.entry(m.clone())
                .and_modify(|d| {
                    if entry.date.as_str() > d.as_str() {
                        *d = entry.date.clone()
                    }
                })
                .or_insert_with(|| entry.date.clone());
        }
    }
    last
}

// ── Parse Claude's response ──────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct CoachResponse {
    name: String,
    #[serde(default)]
    rationale: String,
    exercises: Vec<ScheduledExercise>,
    #[serde(default)]
    vitals: Option<Vitals>,
}

pub fn parse_workout_response(
    json: &str,
    target_date: &str,
    created_at: &str,
    library: &[LibraryExercise],
) -> Result<ParsedResponse, String> {
    // Strip ```json fences if present.
    let trimmed = json.trim();
    let body = if let Some(rest) = trimmed.strip_prefix("```json") {
        rest.trim_start()
            .trim_end_matches("```")
            .trim()
            .to_string()
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest.trim_start()
            .trim_end_matches("```")
            .trim()
            .to_string()
    } else {
        trimmed.to_string()
    };

    let resp: CoachResponse =
        serde_json::from_str(&body).map_err(|e| format!("JSON parse: {e}"))?;
    if resp.exercises.is_empty() {
        return Err("Response had zero exercises".into());
    }
    validate_exercises_against_library(&resp.exercises, library)?;
    let workout = ScheduledWorkout {
        id: uuid::Uuid::new_v4().to_string(),
        date: target_date.to_string(),
        name: resp.name,
        rationale: resp.rationale,
        source: WorkoutSource::Coach,
        exercises: resp.exercises,
        created_at: created_at.to_string(),
    };
    Ok(ParsedResponse {
        workout,
        vitals: resp.vitals,
    })
}

// ── Cardio actuals (Apple Health screenshot → per-zone minutes) ─────────────

/// Per-exercise zone-minute actuals returned by Claude when the user pastes
/// an Apple Health workout summary screenshot.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct CardioActuals {
    /// Library id (preferred) or exercise name to match against the active session.
    #[serde(default)]
    pub exercise_id: Option<String>,
    #[serde(default)]
    pub exercise_name: Option<String>,
    pub zones: Vec<crate::models::ZoneTarget>,
}

#[derive(serde::Deserialize)]
struct CardioActualsResponse {
    cardio_actuals: CardioActuals,
}

/// Parse a fenced-or-raw JSON blob into a CardioActuals. Used by the
/// session view's "paste cardio summary" textarea.
///
/// Accepts both `{"cardio_actuals": {...}}` (preferred — what the prompt asks
/// Claude to return) and the bare unwrapped object. When BOTH parses fail, we
/// prefer the wrapped-form error if the input clearly contains `cardio_actuals`
/// at the top level — otherwise the bare-form fallback's "missing field `zones`"
/// hides the real issue (e.g. a field-type mismatch on a nested value).
pub fn parse_cardio_actuals(json: &str) -> Result<CardioActuals, String> {
    let trimmed = json.trim();
    let body = if let Some(rest) = trimmed.strip_prefix("```json") {
        rest.trim_start().trim_end_matches("```").trim().to_string()
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest.trim_start().trim_end_matches("```").trim().to_string()
    } else {
        trimmed.to_string()
    };
    let wrapped_err = match serde_json::from_str::<CardioActualsResponse>(&body) {
        Ok(wrapped) => return Ok(wrapped.cardio_actuals),
        Err(e) => e,
    };
    let bare_err = match serde_json::from_str::<CardioActuals>(&body) {
        Ok(actuals) => return Ok(actuals),
        Err(e) => e,
    };
    let prefer_wrapped = body.contains("\"cardio_actuals\"");
    let err = if prefer_wrapped { wrapped_err } else { bare_err };
    Err(format!("JSON parse: {err}"))
}

/// Apply parsed vitals to the user's goals, dropping silently if the reading
/// is older than (or equal to) what's already stored. Returns true if applied.
pub fn apply_vitals_to_goals(vitals: &Vitals, goals: &mut UserGoals) -> bool {
    let stale = match goals.vo2_max_updated.as_deref() {
        Some(existing) => vitals.source_date.as_str() <= existing,
        None => false,
    };
    if stale {
        return false;
    }
    goals.vo2_max_latest = Some(vitals.vo2_max);
    goals.vo2_max_updated = Some(vitals.source_date.clone());
    true
}

/// Reject any exercise that doesn't carry a `library_id` matching a real library entry.
/// Returns Ok(()) if every exercise is valid, otherwise an error message listing the offenders.
pub fn validate_exercises_against_library(
    exercises: &[ScheduledExercise],
    library: &[LibraryExercise],
) -> Result<(), String> {
    if library.is_empty() {
        return Err(
            "Exercise library hasn't loaded yet — can't validate. Try again in a moment.".into(),
        );
    }
    let known: HashSet<&str> = library.iter().map(|e| e.id.as_str()).collect();

    let mut missing_id: Vec<String> = Vec::new();
    let mut bad_id: Vec<String> = Vec::new();
    for ex in exercises {
        match ex.library_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            None => missing_id.push(ex.name.clone()),
            Some(id) if !known.contains(id) => bad_id.push(format!("{} (id={id})", ex.name)),
            Some(_) => {}
        }
    }

    if missing_id.is_empty() && bad_id.is_empty() {
        return Ok(());
    }
    let mut msg = String::from("Library validation failed — only exercises from the bundled library are allowed.");
    if !missing_id.is_empty() {
        msg.push_str(&format!(
            "\n  • Missing library_id: {}",
            missing_id.join(", ")
        ));
    }
    if !bad_id.is_empty() {
        msg.push_str(&format!(
            "\n  • Unknown library_id: {}",
            bad_id.join(", ")
        ));
    }
    Err(msg)
}

//! Generates the "coach packet" markdown that gets fed to Claude, and parses
//! the JSON workout it returns.

use std::collections::HashMap;

use crate::library::{days_between, last_hit_by_muscle, recency_bucket, RecencyBucket};
use crate::models::{
    ExerciseEntry, LibraryExercise, ScheduledExercise, ScheduledWorkout, UserGoals,
    WorkoutSource, ALL_MUSCLES,
};

const HISTORY_WINDOW_DAYS: i64 = 14;
const RECENT_SCHEDULED_DAYS: i64 = 7;

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
                        if s.weight > 0.0 {
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

    // ── Library reference ───────────────────────────────────────────────────
    out.push_str("## Exercise library\n\n");
    out.push_str(&format!(
        "The full library ({} exercises) is at `public/data/exercises.json` in the wholesome-swolesome repo, or live at <https://tbaums.github.io/wholesome-swolesome/data/exercises.json>. Each entry has `id`, `name`, `primaryMuscles`, `secondaryMuscles`, `equipment`, `category`, `level`, `mechanic`, `instructions`, and `images`.\n\n",
        input.library.len()
    ));

    // ── Instructions ────────────────────────────────────────────────────────
    out.push_str(
        r#"## Task

Design ONE workout for the target date. Apply:
- **Progressive overload** — if the user hit the top of their rep range on an exercise recently with good completion, bump weight ~2.5% (or one notch). If they missed reps, hold weight.
- **Recovery science** — avoid muscles worked in the last 48h for high-intensity work; touch them only with low-volume accessory work if at all. Prioritize muscles in the "8-14 days" / "never" buckets.
- **Movement balance** — across a week, balance push/pull, knee-dominant / hip-dominant, vertical / horizontal.
- **Volume** — match `session_minutes`. Plan ~10-25 working sets total. Include rest_seconds appropriate for the goal (60-90s for hypertrophy, 180-300s for strength).
- **Equipment** — only prescribe exercises whose `equipment` is in the user's available list.

## Response format

Reply with **ONLY** this JSON, nothing else (no markdown fence, no commentary):

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
    }
  ]
}
```

`library_id` must match a real exercise id from the library. `notes` can be null. Order exercises in the sequence they should be performed.
"#,
    );

    out
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn recent_history<'a>(history: &'a [ExerciseEntry], today: &str, window: i64) -> Vec<&'a ExerciseEntry> {
    history
        .iter()
        .filter(|e| {
            let d = days_between(&e.date, today).unwrap_or(9999);
            (0..=window).contains(&d)
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

// ── Parse Claude's response ──────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct CoachResponse {
    name: String,
    #[serde(default)]
    rationale: String,
    exercises: Vec<ScheduledExercise>,
}

pub fn parse_workout_response(json: &str, target_date: &str, created_at: &str) -> Result<ScheduledWorkout, String> {
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
    Ok(ScheduledWorkout {
        id: uuid::Uuid::new_v4().to_string(),
        date: target_date.to_string(),
        name: resp.name,
        rationale: resp.rationale,
        source: WorkoutSource::Coach,
        exercises: resp.exercises,
        created_at: created_at.to_string(),
    })
}

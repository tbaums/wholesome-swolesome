//! Exercise library loading + history aggregation by muscle group.

use std::collections::HashMap;

use crate::models::{ExerciseEntry, LibraryExercise};

/// Bundled library path (served from /public/data/ via Trunk's copy-dir).
pub const LIBRARY_URL: &str = "data/exercises.json";

#[derive(Debug)]
pub enum LibraryError {
    Network(String),
    Decode(String),
}

impl std::fmt::Display for LibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(s) => write!(f, "network: {s}"),
            Self::Decode(s) => write!(f, "decode: {s}"),
        }
    }
}

pub async fn fetch_library() -> Result<Vec<LibraryExercise>, LibraryError> {
    let resp = gloo_net::http::Request::get(LIBRARY_URL)
        .send()
        .await
        .map_err(|e| LibraryError::Network(e.to_string()))?;
    let text = resp
        .text()
        .await
        .map_err(|e| LibraryError::Network(e.to_string()))?;
    serde_json::from_str(&text).map_err(|e| LibraryError::Decode(e.to_string()))
}

// ── Recency buckets ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecencyBucket {
    /// Worked in the last 3 days — likely still recovering.
    Recent,
    /// Worked 4–7 days ago — near recovery sweet spot.
    Week,
    /// Worked 8–14 days ago — needs attention.
    TwoWeeks,
    /// 15+ days or never — neglected.
    Stale,
}

impl RecencyBucket {
    pub fn color(&self) -> &'static str {
        match self {
            // deep green, light green, yellow, gray
            Self::Recent => "#16a34a",
            Self::Week => "#86efac",
            Self::TwoWeeks => "#facc15",
            Self::Stale => "#374151",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Recent => "≤ 3 days",
            Self::Week => "4–7 days",
            Self::TwoWeeks => "8–14 days",
            Self::Stale => "15+ / never",
        }
    }
}

pub fn recency_bucket(days_since: i64) -> Option<RecencyBucket> {
    if days_since < 0 {
        None
    } else if days_since <= 3 {
        Some(RecencyBucket::Recent)
    } else if days_since <= 7 {
        Some(RecencyBucket::Week)
    } else if days_since <= 14 {
        Some(RecencyBucket::TwoWeeks)
    } else {
        Some(RecencyBucket::Stale)
    }
}

// ── Category lookup ──────────────────────────────────────────────────────────

/// Looks an exercise up in the library by id first, then by case-insensitive
/// name, and returns its category string (e.g. `"cardio"`, `"strength"`).
pub fn category_of<'a>(
    exercise_id: &str,
    exercise_name: &str,
    library: &'a [LibraryExercise],
) -> Option<&'a str> {
    let name_lc = exercise_name.to_lowercase();
    library
        .iter()
        .find(|e| e.id == exercise_id || e.name.to_lowercase() == name_lc)
        .map(|e| e.category.as_str())
}

/// True if the named exercise is in the library with `category == "cardio"`.
/// Used to switch the set-input UI to minutes × intensity instead of weight × reps.
pub fn is_cardio_exercise(
    exercise_id: &str,
    exercise_name: &str,
    library: &[LibraryExercise],
) -> bool {
    category_of(exercise_id, exercise_name, library) == Some("cardio")
}

/// True if the named exercise is in the library with `equipment == "body only"` —
/// i.e., a bodyweight movement (push-ups, pull-ups, dips, plank). Used to hide the
/// weight input from set rows so the user only logs reps. Exercises not found in the
/// library default to false, so freeform / custom entries keep the standard weight × reps
/// inputs unless the user explicitly imports them from a `body only` library entry.
pub fn is_bodyweight_exercise(
    exercise_id: &str,
    exercise_name: &str,
    library: &[LibraryExercise],
) -> bool {
    let name_lc = exercise_name.to_lowercase();
    library
        .iter()
        .find(|e| e.id == exercise_id || e.name.to_lowercase() == name_lc)
        .and_then(|e| e.equipment.as_deref())
        .is_some_and(|eq| eq == "body only")
}

// ── Muscle aggregation from history ──────────────────────────────────────────

/// For each free-exercise-db muscle key, the most recent date (YYYY-MM-DD)
/// it was hit by a completed set. Primary muscles count fully; secondary
/// muscles count too (treated equally for recency — recovery, not volume).
pub fn last_hit_by_muscle(
    history: &[ExerciseEntry],
    library: &[LibraryExercise],
) -> HashMap<String, String> {
    // Index library by both id and lowercased name for fuzzy matches
    let by_id: HashMap<&str, &LibraryExercise> = library.iter().map(|e| (e.id.as_str(), e)).collect();
    let by_name: HashMap<String, &LibraryExercise> =
        library.iter().map(|e| (e.name.to_lowercase(), e)).collect();

    let mut last: HashMap<String, String> = HashMap::new();

    for entry in history {
        // Only count if at least one set was completed.
        if !entry.sets.iter().any(|s| s.completed) {
            continue;
        }
        let muscles = lookup_muscles(entry, &by_id, &by_name);
        for m in muscles {
            last.entry(m)
                .and_modify(|d| if entry.date.as_str() > d.as_str() { *d = entry.date.clone() })
                .or_insert_with(|| entry.date.clone());
        }
    }

    last
}

fn lookup_muscles(
    entry: &ExerciseEntry,
    by_id: &HashMap<&str, &LibraryExercise>,
    by_name: &HashMap<String, &LibraryExercise>,
) -> Vec<String> {
    let lib = by_id
        .get(entry.exercise_id.as_str())
        .or_else(|| by_name.get(&entry.exercise_name.to_lowercase()));
    let Some(lib) = lib else { return Vec::new() };
    let mut out = lib.primary_muscles.clone();
    out.extend(lib.secondary_muscles.iter().cloned());
    out
}

/// Days between two YYYY-MM-DD strings (b - a). Returns None on parse failure.
pub fn days_between(a: &str, b: &str) -> Option<i64> {
    let pa = parse_ymd(a)?;
    let pb = parse_ymd(b)?;
    Some(julian_day(pb) - julian_day(pa))
}

fn parse_ymd(s: &str) -> Option<(i32, u32, u32)> {
    let mut parts = s.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    Some((y, m, d))
}

/// Conway's Doomsday-friendly Julian day number for date math.
fn julian_day((y, m, d): (i32, u32, u32)) -> i64 {
    let (y, m) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let a = y / 100;
    let b = 2 - a + a / 4;
    ((365.25 * (y + 4716) as f64) as i64)
        + ((30.6001 * (m + 1) as f64) as i64)
        + d as i64
        + b as i64
        - 1524
}

use crate::models::{Exercise, ExerciseCategory, ExerciseEntry, WorkoutDay, WorkoutPlan};

// ── Plan CSV ─────────────────────────────────────────────────────────────────
//
// Format (one row per exercise):
//   day_id,day_name,exercise_id,exercise_name,target_sets,reps_min,reps_max,category,notes

pub fn export_plan_csv(plan: &WorkoutPlan) -> String {
    let mut out = String::from("day_id,day_name,exercise_id,exercise_name,target_sets,reps_min,reps_max,category,notes\n");
    for day in &plan.days {
        for ex in &day.exercises {
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}\n",
                csv_field(&day.id),
                csv_field(&day.name),
                csv_field(&ex.id),
                csv_field(&ex.name),
                ex.target_sets,
                ex.reps_min,
                ex.reps_max,
                csv_field(ex.category.label()),
                csv_field(ex.notes.as_deref().unwrap_or("")),
            ));
        }
    }
    out
}

pub fn import_plan_csv(csv: &str) -> Result<WorkoutPlan, String> {
    let mut lines = csv.lines();
    // Skip header
    lines.next();

    let mut days: Vec<WorkoutDay> = Vec::new();

    for (i, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<String> = split_csv_line(line)
            .into_iter()
            .map(unquote_field)
            .collect();
        if cols.len() < 8 {
            return Err(format!("Row {}: expected at least 8 columns, got {}", i + 2, cols.len()));
        }

        let day_id = cols[0].clone();
        let day_name = cols[1].clone();
        let ex_id = cols[2].clone();
        let ex_name = cols[3].clone();
        let target_sets: u32 = cols[4].parse().map_err(|_| format!("Row {}: bad target_sets", i + 2))?;
        let reps_min: u32 = cols[5].parse().map_err(|_| format!("Row {}: bad reps_min", i + 2))?;
        let reps_max: u32 = cols[6].parse().map_err(|_| format!("Row {}: bad reps_max", i + 2))?;
        let category = parse_category(&cols[7]);
        let notes = cols.get(8).cloned().filter(|s| !s.is_empty());

        let exercise = Exercise {
            id: if ex_id.is_empty() {
                uuid::Uuid::new_v4().to_string()
            } else {
                ex_id
            },
            name: ex_name,
            target_sets,
            reps_min,
            reps_max,
            category,
            notes,
        };

        match days.iter_mut().find(|d| d.id == day_id) {
            Some(day) => day.exercises.push(exercise),
            None => days.push(WorkoutDay {
                id: day_id,
                name: day_name,
                exercises: vec![exercise],
            }),
        }
    }

    if days.is_empty() {
        return Err("No exercises found in CSV".into());
    }

    Ok(WorkoutPlan { days })
}

// ── History CSV ───────────────────────────────────────────────────────────────
//
// Format:
//   entry_id,date,day_name,exercise_name,set_number,reps,weight,completed
// day_name is empty for freeform entries.

pub fn export_history_csv(history: &[ExerciseEntry]) -> String {
    let mut out = String::from("entry_id,date,day_name,exercise_name,set_number,reps,weight,completed\n");
    for entry in history {
        let day_name = entry.day_name.as_deref().unwrap_or("");
        for set in &entry.sets {
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                csv_field(&entry.id),
                csv_field(&entry.date),
                csv_field(day_name),
                csv_field(&entry.exercise_name),
                set.set_number,
                set.reps,
                set.weight,
                set.completed,
            ));
        }
    }
    out
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Wraps a field in quotes if it contains a comma or quote.
fn csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Strips whitespace, removes surrounding `"` if present, and collapses the
/// CSV-standard `""` escape sequence into a literal `"`. Round-trips with
/// `csv_field` above.
fn unquote_field(s: &str) -> String {
    let s = s.trim();
    let bytes = s.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
        s[1..s.len() - 1].replace("\"\"", "\"")
    } else {
        s.to_string()
    }
}

/// Naive CSV line splitter (handles quoted fields).
fn split_csv_line(line: &str) -> Vec<&str> {
    let mut fields = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_quotes = !in_quotes,
            b',' if !in_quotes => {
                fields.push(&line[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    fields.push(&line[start..]);
    fields
}

fn parse_category(s: &str) -> ExerciseCategory {
    match s.to_lowercase().as_str() {
        "core" => ExerciseCategory::Core,
        "cardio" => ExerciseCategory::Cardio,
        _ => ExerciseCategory::Main,
    }
}

/// Triggers a browser download of `content` as a file named `filename`.
pub fn download_file(filename: &str, content: &str) {
    use wasm_bindgen::JsCast;
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    let blob_parts = js_sys::Array::new();
    blob_parts.push(&wasm_bindgen::JsValue::from_str(content));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("text/csv;charset=utf-8");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(
        &blob_parts.into(),
        &opts,
    )
    .expect("blob creation failed");

    let url = web_sys::Url::create_object_url_with_blob(&blob).expect("create_object_url failed");

    let anchor = document
        .create_element("a")
        .expect("create element failed")
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .expect("cast failed");

    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.click();

    let _ = web_sys::Url::revoke_object_url(&url);
}

use crate::models::ExerciseEntry;

// ── History CSV ───────────────────────────────────────────────────────────────
//
// Format:
//   entry_id,date,day_name,exercise_name,set_number,reps,weight,completed
// day_name is empty for freeform entries.

pub fn export_history_csv(history: &[ExerciseEntry]) -> String {
    let mut out = String::from("entry_id,date,day_name,exercise_name,set_number,reps,weight,duration_seconds,completed\n");
    for entry in history {
        let day_name = entry.day_name.as_deref().unwrap_or("");
        for set in &entry.sets {
            let dur = set.duration_seconds.map(|d| d.to_string()).unwrap_or_default();
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}\n",
                csv_field(&entry.id),
                csv_field(&entry.date),
                csv_field(day_name),
                csv_field(&entry.exercise_name),
                set.set_number,
                set.reps,
                set.weight,
                dur,
                set.completed,
            ));
        }
    }
    out
}

fn csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
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
    opts.set_type("text/plain;charset=utf-8");
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

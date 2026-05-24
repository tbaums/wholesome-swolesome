use leptos::prelude::*;

use crate::app::{AppState, View};

/// Shows per-exercise history: every set ever logged for that exercise,
/// grouped by session date so you can track weight progression over time.
#[component]
pub fn ProgressView(exercise_name: String) -> impl IntoView {
    let state = expect_context::<AppState>();

    // Gather all exercise entries for this exercise name
    let rows = {
        let exercise_name = exercise_name.clone();
        move || {
            let history = state.history.get();
            let mut rows: Vec<(String, String, u32, f32, bool)> = Vec::new(); // (date, _tag, reps, weight, completed)
            for entry in &history {
                if entry.exercise_name == exercise_name {
                    for set in &entry.sets {
                        rows.push((
                            entry.date.clone(),
                            entry.day_name.clone().unwrap_or_else(|| "Freeform".to_string()),
                            set.reps,
                            set.weight,
                            set.completed,
                        ));
                    }
                }
            }
            // Newest first
            rows.sort_by(|a, b| b.0.cmp(&a.0));
            rows
        }
    };

    let is_cardio = {
        let exercise_name = exercise_name.clone();
        move || {
            let history = state.history.get();
            let exercise_id = history.iter()
                .find(|e| e.exercise_name == exercise_name)
                .map(|e| e.exercise_id.clone())
                .unwrap_or_default();
            crate::library::is_cardio_exercise(
                &exercise_id,
                &exercise_name,
                &state.library.get(),
            )
        }
    };

    // Best set ever:
    //   strength → max weight among completed sets ("100 × 5 reps on …")
    //   cardio   → max minutes among completed sets ("30 min @ RPE 6 on …")
    let rows_for_best = rows.clone();
    let is_cardio_for_best = is_cardio.clone();
    let best = move || {
        let cardio = is_cardio_for_best();
        let data = rows_for_best();
        let completed = data.iter().filter(|(_, _, _, _, done)| *done);
        if cardio {
            completed
                .max_by_key(|r| r.2)  // r.2 = reps = minutes
                .map(|(date, _, reps, weight, _)| {
                    format!("{} min @ RPE {:.0} on {}", reps, weight, date)
                })
        } else {
            completed
                .max_by(|a, b| a.3.partial_cmp(&b.3).unwrap())
                .map(|(date, _, reps, weight, _)| {
                    format!("{:.1} × {} reps on {}", weight, reps, date)
                })
        }
    };

    let is_cardio_for_table = is_cardio.clone();

    view! {
        <div class="page">
            <div class="page-header">
                <button class="back-btn" on:click=move |_| state.navigate(View::History)>
                    "‹ Back"
                </button>
                <div>
                    <div class="page-title">{exercise_name.clone()}</div>
                    <div class="text-muted text-sm">"Progress"</div>
                </div>
            </div>

            {move || best().map(|b| view! {
                <div class="card" style="border-left: 3px solid var(--accent)">
                    <div class="text-sm text-muted">"Personal best"</div>
                    <div class="fw-600 text-accent">{b}</div>
                </div>
            })}

            {move || {
                let data = rows();
                if data.is_empty() {
                    view! {
                        <div class="empty">
                            <div class="empty-icon">"📊"</div>
                            <div>"No data yet for this exercise."</div>
                        </div>
                    }.into_any()
                } else {
                    let rows2 = rows.clone();
                    let cardio = is_cardio_for_table();
                    let (col1, col2) = if cardio { ("Min", "Intensity") } else { ("Weight", "Reps") };
                    view! {
                        <div class="card">
                            <table class="progress-table" style="width:100%">
                                <thead>
                                    <tr>
                                        <th>"Date"</th>
                                        <th>{col1}</th>
                                        <th>{col2}</th>
                                        <th>"Done"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <For
                                        each=rows2
                                        key=|r| format!("{}{}{}{:.1}", r.0, r.2, r.3, r.4)
                                        children=move |(date, _day, reps, weight, done)| {
                                            let (v1, v2) = if cardio {
                                                (reps.to_string(), format!("{:.0}", weight))
                                            } else {
                                                (format!("{:.1}", weight), reps.to_string())
                                            };
                                            view! {
                                                <tr>
                                                    <td class="text-sm">{date}</td>
                                                    <td>{v1}</td>
                                                    <td>{v2}</td>
                                                    <td style={if done { "color:var(--success)" } else { "color:var(--text-muted)" }}>
                                                        {if done { "✓" } else { "—" }}
                                                    </td>
                                                </tr>
                                            }
                                        }
                                    />
                                </tbody>
                            </table>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

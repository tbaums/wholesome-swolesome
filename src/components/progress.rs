use leptos::prelude::*;

use crate::app::{AppState, View};

/// Shows per-exercise history: every set ever logged for that exercise,
/// grouped by session date so you can track weight progression over time.
#[component]
pub fn ProgressView(exercise_name: String) -> impl IntoView {
    let state = expect_context::<AppState>();

    // Gather all sessions that contain this exercise
    let rows = {
        let exercise_name = exercise_name.clone();
        move || {
            let history = state.history.get();
            let mut rows: Vec<(String, String, u32, f32, bool)> = Vec::new(); // (date, session_id, reps, weight, completed)
            for session in &history {
                for log in &session.exercise_logs {
                    if log.exercise_name == exercise_name {
                        for set in &log.sets {
                            rows.push((
                                session.date.clone(),
                                session.day_name.clone(),
                                set.reps,
                                set.weight_lbs,
                                set.completed,
                            ));
                        }
                    }
                }
            }
            // Newest first
            rows.sort_by(|a, b| b.0.cmp(&a.0));
            rows
        }
    };

    // Best set ever (max weight among completed sets)
    // Clone rows so `best` and the reactive render section each get their own copy.
    let rows_for_best = rows.clone();
    let best = move || {
        rows_for_best().iter()
            .filter(|(_, _, _, _, done)| *done)
            .max_by(|a, b| a.3.partial_cmp(&b.3).unwrap())
            .map(|(date, _, reps, weight, _)| format!("{:.1} lbs × {} reps on {}", weight, reps, date))
    };

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
                    // Clone so the outer Fn closure isn't consumed by the inner move ||
                    let rows2 = rows.clone();
                    view! {
                        <div class="card">
                            <table class="progress-table" style="width:100%">
                                <thead>
                                    <tr>
                                        <th>"Date"</th>
                                        <th>"Weight"</th>
                                        <th>"Reps"</th>
                                        <th>"Done"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <For
                                        each=rows2
                                        key=|r| format!("{}{}{}{:.1}", r.0, r.2, r.3, r.4)
                                        children=|(date, _day, reps, weight, done)| view! {
                                            <tr>
                                                <td class="text-sm">{date}</td>
                                                <td>{format!("{:.1}", weight)}</td>
                                                <td>{reps}</td>
                                                <td style={if done { "color:var(--success)" } else { "color:var(--text-muted)" }}>
                                                    {if done { "✓" } else { "—" }}
                                                </td>
                                            </tr>
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

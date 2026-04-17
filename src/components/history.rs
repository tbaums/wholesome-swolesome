use leptos::prelude::*;

use crate::app::{AppState, View};
use crate::csv_utils::{download_file, export_history_csv};

#[component]
pub fn HistoryView() -> impl IntoView {
    let state = expect_context::<AppState>();

    let sorted_history = move || {
        let mut h = state.history.get();
        h.sort_by(|a, b| b.date.cmp(&a.date));
        h
    };

    let export = move |_| {
        let csv = export_history_csv(&state.history.get());
        download_file("workout_history.csv", &csv);
    };

    view! {
        <div class="page">
            <div class="page-header">
                <h1 class="page-title">"History"</h1>
                <button class="btn btn-secondary btn-sm" style="margin-left:auto" on:click=export>
                    "Export CSV"
                </button>
            </div>

            {move || {
                let sessions = sorted_history();
                if sessions.is_empty() {
                    view! {
                        <div class="empty">
                            <div class="empty-icon">"📭"</div>
                            <div>"No workout history yet."</div>
                            <div class="text-sm mt-8">"Complete a session to see it here."</div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div>
                            <For
                                each=sorted_history
                                key=|s| s.id.clone()
                                children=move |session| {
                                    let session_id = session.id.clone();
                                    let completed_sets: usize = session.exercise_logs.iter()
                                        .map(|e| e.sets.iter().filter(|s| s.completed).count())
                                        .sum();
                                    let total_sets: usize = session.exercise_logs.iter()
                                        .map(|e| e.sets.len())
                                        .sum();
                                    view! {
                                        <div
                                            class="card"
                                            style="cursor:pointer; margin-bottom:8px"
                                            on:click=move |_| state.navigate(View::SessionDetail { session_id: session_id.clone() })
                                        >
                                            <div class="history-item">
                                                <div>
                                                    <div class="fw-600">{session.day_name}</div>
                                                    <div class="history-date">{session.date}</div>
                                                </div>
                                                <div class="history-stats">
                                                    <div>{session.exercise_logs.len()} " exercises"</div>
                                                    <div>{completed_sets} "/" {total_sets} " sets"</div>
                                                </div>
                                            </div>
                                        </div>
                                    }
                                }
                            />
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

// ── Session detail ────────────────────────────────────────────────────────────

#[component]
pub fn SessionDetailView(session_id: String) -> impl IntoView {
    let state = expect_context::<AppState>();

    let session = {
        let session_id = session_id.clone();
        move || {
            state.history.get()
                .into_iter()
                .find(|s| s.id == session_id)
        }
    };

    let delete_session = {
        let session_id = session_id.clone();
        move |_| {
            state.history.update(|h| h.retain(|s| s.id != session_id));
            state.navigate(View::History);
            state.show_toast("Session deleted");
        }
    };

    view! {
        <div class="page">
            <div class="page-header">
                <button class="back-btn" on:click=move |_| state.navigate(View::History)>
                    "‹ Back"
                </button>
                {move || session().map(|s| view! {
                    <div>
                        <div class="page-title">{s.day_name}</div>
                        <div class="text-muted text-sm">{s.date}</div>
                    </div>
                })}
            </div>

            {move || match session() {
                None => view! { <p class="text-muted">"Session not found."</p> }.into_any(),
                Some(_) => {
                    view! {
                        <div>
                            <For
                                each=move || session().map(|s| s.exercise_logs).unwrap_or_default()
                                key=|log| log.exercise_id.clone()
                                children=move |log| {
                                    let ex_name = log.exercise_name.clone();
                                    let sets = log.sets.clone();
                                    let completed = sets.iter().filter(|s| s.completed).count();
                                    let total = sets.len();
                                    view! {
                                        <div class="card" style="margin-bottom:12px">
                                            <div class="card-title">{ex_name}</div>
                                            <div class="card-sub" style="margin-bottom:10px">
                                                {completed} " / " {total} " sets completed"
                                            </div>
                                            <table class="progress-table">
                                                <thead>
                                                    <tr>
                                                        <th>"Set"</th>
                                                        <th>"Weight (lbs)"</th>
                                                        <th>"Reps"</th>
                                                        <th>"Done"</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    <For
                                                        each=move || sets.clone()
                                                        key=|s| s.set_number
                                                        children=|set| view! {
                                                            <tr>
                                                                <td>{set.set_number}</td>
                                                                <td>{format!("{:.1}", set.weight_lbs)}</td>
                                                                <td>{set.reps}</td>
                                                                <td>{if set.completed { "✓" } else { "—" }}</td>
                                                            </tr>
                                                        }
                                                    />
                                                </tbody>
                                            </table>
                                            // Link to exercise progress
                                            <button
                                                class="btn btn-ghost btn-sm"
                                                style="margin-top:8px; padding-left:0"
                                                on:click={
                                                    let name = log.exercise_name.clone();
                                                    move |_| state.navigate(View::Progress { exercise_name: name.clone() })
                                                }
                                            >
                                                "View progress →"
                                            </button>
                                        </div>
                                    }
                                }
                            />
                            <button class="btn btn-danger btn-full" style="margin-top:8px" on:click=delete_session>
                                "Delete Session"
                            </button>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

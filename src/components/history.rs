use leptos::prelude::*;

use crate::app::{AppState, View};
use crate::csv_utils::{download_file, export_history_csv};

// ── History list ──────────────────────────────────────────────────────────────

#[component]
pub fn HistoryView() -> impl IntoView {
    let state = expect_context::<AppState>();

    let sorted_entries = move || {
        let mut h = state.history.get();
        h.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));
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
                let entries = sorted_entries();
                if entries.is_empty() {
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
                                each=sorted_entries
                                key=|e| e.id.clone()
                                children=move |entry| {
                                    let entry_id = entry.id.clone();
                                    let completed = entry.sets.iter().filter(|s| s.completed).count();
                                    let total = entry.sets.len();
                                    let tag = entry.day_name.clone()
                                        .unwrap_or_else(|| "Freeform".to_string());
                                    view! {
                                        <div
                                            class="card history-item"
                                            style="cursor:pointer; margin-bottom:8px"
                                            on:click=move |_| state.navigate(View::SessionDetail {
                                                session_id: entry_id.clone()
                                            })
                                        >
                                            <div style="display:flex; justify-content:space-between; align-items:flex-start">
                                                <div>
                                                    <div class="fw-600">{entry.exercise_name}</div>
                                                    <div class="history-date">{entry.date}</div>
                                                    <div class="text-muted text-sm" style="margin-top:2px">
                                                        {tag}
                                                    </div>
                                                </div>
                                                <div class="history-stats">
                                                    <div>{completed} "/" {total} " sets"</div>
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

// ── Entry detail ──────────────────────────────────────────────────────────────

#[component]
pub fn SessionDetailView(session_id: String) -> impl IntoView {
    let state = expect_context::<AppState>();

    let entry = {
        let session_id = session_id.clone();
        move || {
            state.history.get()
                .into_iter()
                .find(|e| e.id == session_id)
        }
    };

    let delete_entry = {
        let session_id = session_id.clone();
        move |_| {
            state.history.update(|h| h.retain(|e| e.id != session_id));
            state.navigate(View::History);
            state.show_toast("Entry deleted");
        }
    };

    let entry_header = entry.clone();
    let entry_body = entry.clone();

    view! {
        <div class="page">
            <div class="page-header">
                <button class="back-btn" on:click=move |_| state.navigate(View::History)>
                    "‹ Back"
                </button>
                {move || entry_header().map(|e| view! {
                    <div>
                        <div class="page-title">{e.exercise_name}</div>
                        <div class="text-muted text-sm">
                            {e.day_name.unwrap_or_else(|| "Freeform".to_string())}
                            " · " {e.date}
                        </div>
                    </div>
                })}
            </div>

            {move || match entry_body() {
                None => view! { <p class="text-muted">"Entry not found."</p> }.into_any(),
                Some(e) => {
                    let sets = e.sets.clone();
                    let completed = sets.iter().filter(|s| s.completed).count();
                    let total = sets.len();
                    let exercise_name = e.exercise_name.clone();
                    let delete_entry = delete_entry.clone();
                    view! {
                        <div>
                            <div class="card" style="margin-bottom:12px">
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
                                <button
                                    class="btn btn-ghost btn-sm"
                                    style="margin-top:8px; padding-left:0"
                                    on:click={
                                        let name = exercise_name.clone();
                                        move |_| state.navigate(View::Progress { exercise_name: name.clone() })
                                    }
                                >
                                    "View progress →"
                                </button>
                            </div>
                            <button
                                class="btn btn-danger btn-full"
                                style="margin-top:8px"
                                on:click=delete_entry
                            >
                                "Delete Entry"
                            </button>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

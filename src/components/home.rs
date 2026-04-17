use leptos::prelude::*;

use crate::app::{new_session, AppState, View};

#[component]
pub fn HomeView() -> impl IntoView {
    let state = expect_context::<AppState>();

    view! {
        <div class="page">
            <div class="page-header">
                <h1 class="page-title">"Wholesome Swolesome 💪"</h1>
            </div>

            <DayGrid/>
            <RecentSessions/>
        </div>
    }
}

// ── Day selection grid ────────────────────────────────────────────────────────

#[component]
fn DayGrid() -> impl IntoView {
    let state = expect_context::<AppState>();
    let plan = state.plan;
    let history = state.history;

    // Find the day_id of the last completed session to suggest the next one
    let suggested_day_id = move || {
        let h = history.get();
        let last = h.iter().rev().find(|s| s.is_complete);
        last.map(|s| s.day_id.clone())
    };

    view! {
        <div class="card">
            <div class="card-title">"Select Today's Workout"</div>
            <div class="card-sub" style="margin-bottom:8px">"Tap a day to begin"</div>
            <div style="margin-top:12px">
                <For
                    each=move || plan.get().days
                    key=|day| day.id.clone()
                    children=move |day| {
                        let day_id = day.id.clone();
                        let is_suggested = {
                            let day_id = day_id.clone();
                            move || {
                                // Suggest the day *after* the last completed one
                                let plan = plan.get();
                                if let Some(last_id) = suggested_day_id() {
                                    let last_pos = plan.days.iter().position(|d| d.id == last_id);
                                    if let Some(pos) = last_pos {
                                        let next_pos = (pos + 1) % plan.days.len();
                                        return plan.days[next_pos].id == day_id;
                                    }
                                }
                                // If no history, suggest day 1
                                plan.days.first().map(|d| d.id == day_id).unwrap_or(false)
                            }
                        };
                        let ex_count = day.exercises.len();
                        let on_start = {
                            let day_id = day_id.clone();
                            move |_| {
                                let session = new_session(
                                    &day_id,
                                    &state.plan.get(),
                                    &state.history.get(),
                                );
                                if let Some(s) = session {
                                    state.active_session.set(Some(s));
                                    state.navigate(View::Session { day_id: day_id.clone() });
                                }
                            }
                        };
                        view! {
                            <button
                                class="btn btn-secondary btn-full"
                                style="justify-content:space-between; margin-bottom:8px"
                                on:click=on_start
                            >
                                <span style="display:flex; flex-direction:column; align-items:flex-start; gap:2px">
                                    <span>{day.name}</span>
                                    <span class="text-muted text-sm">
                                        {ex_count} " exercises"
                                    </span>
                                </span>
                                {move || is_suggested().then(|| view! {
                                    <span style="background:var(--accent); color:#fff; font-size:11px; padding:3px 8px; border-radius:20px; font-weight:600">
                                        "Suggested"
                                    </span>
                                })}
                            </button>
                        }
                    }
                />
            </div>
        </div>
    }
}

// ── Recent sessions ───────────────────────────────────────────────────────────

#[component]
fn RecentSessions() -> impl IntoView {
    let state = expect_context::<AppState>();

    let recent = move || {
        let mut h = state.history.get();
        h.sort_by(|a, b| b.date.cmp(&a.date));
        h.into_iter().take(5).collect::<Vec<_>>()
    };

    view! {
        <div class="card">
            <div class="card-title">"Recent Sessions"</div>
            {move || {
                let sessions = recent();
                if sessions.is_empty() {
                    view! {
                        <div class="empty">
                            <div class="empty-icon">"📭"</div>
                            <div>"No sessions yet. Start your first workout!"</div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div>
                            <For
                                each=move || recent()
                                key=|s| s.id.clone()
                                children=move |session| {
                                    let session_id = session.id.clone();
                                    let completed_sets: usize = session.exercise_logs.iter()
                                        .map(|e| e.sets.iter().filter(|s| s.completed).count())
                                        .sum();
                                    view! {
                                        <div
                                            class="history-item card"
                                            style="cursor:pointer; margin-bottom:8px"
                                            on:click=move |_| state.navigate(View::SessionDetail { session_id: session_id.clone() })
                                        >
                                            <div>
                                                <div class="fw-600">{session.day_name}</div>
                                                <div class="history-date">{session.date}</div>
                                            </div>
                                            <div class="history-stats">
                                                <div>{session.exercise_logs.len()} " exercises"</div>
                                                <div>{completed_sets} " sets done"</div>
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

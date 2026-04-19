use leptos::prelude::*;

use crate::app::{new_session, AppState, View};

#[component]
pub fn HomeView() -> impl IntoView {
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

    let enumerated_days = move || {
        plan.get().days.into_iter().enumerate().collect::<Vec<_>>()
    };

    view! {
        <div class="card">
            <div class="card-title">"Select Today's Workout"</div>
            <div class="card-sub" style="margin-bottom:8px">"Tap a day to begin"</div>
            <div style="margin-top:12px">
                <For
                    each=enumerated_days
                    key=|(_, day)| day.id.clone()
                    children=move |(idx, day)| {
                        let day_id = day.id.clone();
                        let day_num = idx + 1;
                        let ex_count = day.exercises.len();
                        let on_start = {
                            let day_id = day_id.clone();
                            move |_| {
                                let existing = state.active_session.get_untracked();

                                // Same day already active — just resume
                                if let Some(ref s) = existing {
                                    if s.day_id == day_id {
                                        state.navigate(View::Session { day_id: day_id.clone() });
                                        return;
                                    }
                                    // Different day — shelve current session to drafts
                                    state.session_drafts.update(|drafts| {
                                        let s = existing.clone().unwrap();
                                        if let Some(pos) = drafts.iter().position(|d| d.day_id == s.day_id) {
                                            drafts[pos] = s;
                                        } else {
                                            drafts.push(s);
                                        }
                                    });
                                    state.active_session.set(None);
                                }

                                // Check drafts for the requested day
                                let draft = state.session_drafts.get_untracked()
                                    .into_iter()
                                    .find(|d| d.day_id == day_id);

                                if let Some(d) = draft {
                                    state.session_drafts.update(|drafts| drafts.retain(|d| d.day_id != day_id));
                                    state.active_session.set(Some(d));
                                } else {
                                    let session = new_session(&day_id, &state.plan.get(), &state.history.get());
                                    if let Some(s) = session {
                                        state.active_session.set(Some(s));
                                    }
                                }
                                state.navigate(View::Session { day_id: day_id.clone() });
                            }
                        };
                        view! {
                            <button
                                class="btn btn-secondary btn-full"
                                style="justify-content:space-between; margin-bottom:8px"
                                on:click=on_start
                            >
                                <span style="display:flex; flex-direction:column; align-items:flex-start; gap:1px">
                                    <span style="font-size:11px; color:var(--text-muted); font-weight:500">
                                        "Day " {day_num}
                                    </span>
                                    <span>{day.name}</span>
                                    <span class="text-muted text-sm">{ex_count} " exercises"</span>
                                </span>
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

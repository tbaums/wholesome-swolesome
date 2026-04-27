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

/// A deduplicated session group derived from flat ExerciseEntry history.
#[derive(Clone)]
struct SessionGroup {
    key: String,       // session_id, or "freeform-{date}"
    label: String,     // day name or "Freeform"
    date: String,
    n_exercises: usize,
    completed_sets: usize,
}

#[component]
fn RecentSessions() -> impl IntoView {
    let state = expect_context::<AppState>();

    let recent = move || {
        let entries = state.history.get();
        let mut sorted = entries.clone();
        sorted.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));

        let mut seen = std::collections::HashSet::new();
        let mut groups: Vec<SessionGroup> = Vec::new();

        for entry in &sorted {
            let key = entry.session_id.clone()
                .unwrap_or_else(|| format!("freeform-{}", entry.date));
            if !seen.insert(key.clone()) { continue; }

            let group_entries: Vec<_> = entries.iter().filter(|e| {
                match (&entry.session_id, &e.session_id) {
                    (Some(s1), Some(s2)) => s1 == s2,
                    (None, None) => e.date == entry.date,
                    _ => false,
                }
            }).collect();

            groups.push(SessionGroup {
                key,
                label: entry.day_name.clone().unwrap_or_else(|| "Freeform".to_string()),
                date: entry.date.clone(),
                n_exercises: group_entries.len(),
                completed_sets: group_entries.iter()
                    .flat_map(|e| e.sets.iter())
                    .filter(|s| s.completed)
                    .count(),
            });

            if groups.len() >= 5 { break; }
        }
        groups
    };

    view! {
        <div class="card">
            <div class="card-title">"Recent Sessions"</div>
            {move || {
                let groups = recent();
                if groups.is_empty() {
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
                                key=|g| g.key.clone()
                                children=move |group| {
                                    view! {
                                        <div
                                            class="history-item card"
                                            style="cursor:pointer; margin-bottom:8px"
                                            on:click=move |_| state.navigate(View::History)
                                        >
                                            <div>
                                                <div class="fw-600">{group.label}</div>
                                                <div class="history-date">{group.date}</div>
                                            </div>
                                            <div class="history-stats">
                                                <div>{group.n_exercises} " exercises"</div>
                                                <div>{group.completed_sets} " sets done"</div>
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

use leptos::prelude::*;

use crate::app::{current_date, new_session_from_scheduled, AppState, View};
use crate::models::{ExerciseEntry, ScheduledWorkout};

/// True if any history entry references this scheduled workout via day_id
/// AND has at least one completed set. Used by Home to hide today's card
/// once the workout has been logged, so the user is nudged to plan the next
/// one rather than re-entering the same session.
pub fn is_workout_completed_in_history(workout_id: &str, history: &[ExerciseEntry]) -> bool {
    history.iter().any(|e| {
        e.day_id.as_deref() == Some(workout_id)
            && e.sets.iter().any(|s| s.completed)
    })
}

#[component]
pub fn HomeView() -> impl IntoView {
    view! {
        <div class="page">
            <div class="page-header">
                <h1 class="page-title">"Wholesome Swolesome 💪"</h1>
            </div>

            <TodayCard/>
            <UpcomingList/>
            <CoachActions/>
            <RecentSessions/>
        </div>
    }
}

// ── Today's workout ──────────────────────────────────────────────────────────

/// Lookup result for today's slot: a workout to run, a workout already done,
/// or nothing planned at all.
enum TodaySlot {
    Pending(ScheduledWorkout),
    Done(String), // completed workout name (for the "done" celebration)
    Empty,
}

#[component]
fn TodayCard() -> impl IntoView {
    let state = expect_context::<AppState>();

    let today_slot = move || -> TodaySlot {
        let today = current_date();
        let history = state.history.get();
        let today_scheduled = state
            .scheduled_workouts
            .get()
            .into_iter()
            .find(|w| w.date == today);
        match today_scheduled {
            Some(w) if is_workout_completed_in_history(&w.id, &history) => TodaySlot::Done(w.name),
            Some(w) => TodaySlot::Pending(w),
            None => TodaySlot::Empty,
        }
    };

    view! {
        {move || match today_slot() {
            TodaySlot::Pending(w) => view! { <ScheduledCard workout=w label="TODAY"/> }.into_any(),
            TodaySlot::Done(name) => view! {
                <div class="today-card">
                    <span class="today-badge">"DONE"</span>
                    <div class="today-title">"✓ " {name}</div>
                    <div class="today-rationale">
                        "Today's workout is logged. Generate tomorrow's now, or wait for the nightly coach run."
                    </div>
                </div>
            }.into_any(),
            TodaySlot::Empty => view! {
                <div class="today-card">
                    <span class="today-badge">"TODAY"</span>
                    <div class="today-title">"No workout scheduled"</div>
                    <div class="today-rationale">
                        "The coach hasn't planned today's session yet. Generate one now or check back after midnight."
                    </div>
                </div>
            }.into_any(),
        }}
    }
}

#[component]
fn ScheduledCard(workout: ScheduledWorkout, #[prop()] label: &'static str) -> impl IntoView {
    let state = expect_context::<AppState>();
    let w_clone = workout.clone();
    let start = move |_| {
        let workout_id = w_clone.id.clone();
        let existing = state.active_session.get_untracked();

        if let Some(ref s) = existing {
            if s.day_id == workout_id {
                state.navigate(View::Session { workout_id });
                return;
            }
        }

        let history = state.history.get_untracked();
        let session = new_session_from_scheduled(&w_clone, &history);

        if let Some(s) = existing {
            state.session_drafts.update(|drafts| {
                if let Some(pos) = drafts.iter().position(|d| d.day_id == s.day_id) {
                    drafts[pos] = s;
                } else {
                    drafts.push(s);
                }
            });
        }
        // Resume from draft if present
        let draft = state
            .session_drafts
            .get_untracked()
            .into_iter()
            .find(|d| d.day_id == workout_id);
        if let Some(d) = draft {
            state
                .session_drafts
                .update(|drafts| drafts.retain(|d| d.day_id != workout_id));
            state.active_session.set(Some(d));
        } else {
            state.active_session.set(Some(session));
        }
        state.navigate(View::Session { workout_id });
    };

    let rationale = workout.rationale.clone();
    let date = workout.date.clone();
    let name = workout.name.clone();
    let exercises = workout.exercises.clone();

    view! {
        <div class="today-card">
            <span class="today-badge">{label}</span>
            <div class="today-title">{name}</div>
            <div class="text-muted text-sm" style="margin-bottom:8px">{date}</div>
            {(!rationale.is_empty()).then(|| view! {
                <div class="today-rationale">{rationale}</div>
            })}
            <ul class="today-ex-list">
                {exercises.iter().map(|ex| {
                    let prescription = if let Some(dur) = ex.target_duration_seconds {
                        format!("{}×{}s", ex.target_sets, dur)
                    } else {
                        format!("{}×{}-{}", ex.target_sets, ex.reps_min, ex.reps_max)
                    };
                    view! {
                        <li class="today-ex-row">
                            <span class="today-ex-name">{ex.name.clone()}</span>
                            <span class="today-ex-prescription">{prescription}</span>
                        </li>
                    }
                }).collect_view()}
            </ul>
            <button class="btn btn-finish btn-full" on:click=start>
                {move || {
                    let is_active = state.active_session.get()
                        .map(|s| s.day_id == workout.id)
                        .unwrap_or(false);
                    if is_active { "Resume workout →" } else { "Start workout →" }
                }}
            </button>
        </div>
    }
}

// ── Upcoming ─────────────────────────────────────────────────────────────────

#[component]
fn UpcomingList() -> impl IntoView {
    let state = expect_context::<AppState>();

    let upcoming = move || {
        let today = current_date();
        let mut v: Vec<ScheduledWorkout> = state
            .scheduled_workouts
            .get()
            .into_iter()
            .filter(|w| w.date.as_str() > today.as_str())
            .collect();
        v.sort_by(|a, b| a.date.cmp(&b.date));
        v.into_iter().take(3).collect::<Vec<_>>()
    };

    view! {
        {move || {
            let list = upcoming();
            if list.is_empty() { ().into_any() } else {
                view! {
                    <div class="card">
                        <div class="card-title">"Upcoming"</div>
                        {list.into_iter().map(|w| {
                            let prescription = format!("{} exercises", w.exercises.len());
                            view! {
                                <div class="history-item" style="padding:8px 0; border-top:1px solid var(--border)">
                                    <div>
                                        <div class="fw-600">{w.name.clone()}</div>
                                        <div class="history-date">{w.date.clone()}</div>
                                    </div>
                                    <div class="history-stats">{prescription}</div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }
        }}
    }
}

// ── Coach actions ────────────────────────────────────────────────────────────

#[component]
fn CoachActions() -> impl IntoView {
    let state = expect_context::<AppState>();
    view! {
        <button
            class="btn btn-secondary btn-full"
            style="margin-bottom:8px"
            on:click=move |_| state.navigate(View::CoachPacket)
        >
            "🧠  Generate workout with Claude"
        </button>
    }
}

// ── Recent sessions ──────────────────────────────────────────────────────────

#[derive(Clone)]
struct SessionGroup {
    key: String,
    label: String,
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

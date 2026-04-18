use leptos::prelude::*;

use crate::app::{AppState, View};

// ── Session view ──────────────────────────────────────────────────────────────

#[component]
pub fn SessionView(_day_id: String) -> impl IntoView {
    let state = expect_context::<AppState>();

    // Memo only fires when None↔Some flips — not on every set toggle —
    // so ActiveSession is never torn down and re-created mid-workout.
    let has_session = Memo::new(move |_| state.active_session.get().is_some());

    move || match has_session.get() {
        false => view! {
            <div class="page">
                <p class="text-muted">"No active session."</p>
                <button class="btn btn-secondary mt-16"
                    on:click=move |_| state.navigate(View::Home)>
                    "← Back"
                </button>
            </div>
        }.into_any(),
        true => view! { <ActiveSession/> }.into_any(),
    }
}

// ── Active session ────────────────────────────────────────────────────────────

#[component]
fn ActiveSession() -> impl IntoView {
    let state = expect_context::<AppState>();

    // Tracks which exercise (by ID) is currently expanded — accordion behaviour.
    let open_ex: RwSignal<Option<String>> = RwSignal::new(None);

    let day_name = move || {
        state.active_session.get()
            .map(|s| s.day_name.clone())
            .unwrap_or_default()
    };

    let date = move || {
        state.active_session.get()
            .map(|s| s.date.clone())
            .unwrap_or_default()
    };

    let all_done = move || {
        state.active_session.get()
            .map(|s| {
                !s.exercise_logs.is_empty()
                    && s.exercise_logs.iter().all(|e| {
                        !e.sets.is_empty() && e.sets.iter().all(|set| set.completed)
                    })
            })
            .unwrap_or(false)
    };

    let finish = move |_| {
        state.active_session.update(|opt| {
            if let Some(s) = opt.as_mut() {
                s.is_complete = true;
            }
        });
        if let Some(session) = state.active_session.get() {
            state.history.update(|h| {
                if let Some(pos) = h.iter().position(|s| s.id == session.id) {
                    h[pos] = session.clone();
                } else {
                    h.push(session.clone());
                }
            });
            state.active_session.set(None);
            state.show_toast("Workout saved! 💪");
            state.navigate(View::History);
        }
    };

    let discard = move |_| {
        state.active_session.set(None);
        state.navigate(View::Home);
    };

    // Compute once — exercises don't change mid-session, and keeping this
    // non-reactive stops `For` from re-creating cards on every set toggle
    // (which was causing the accordion to collapse on each interaction).
    let exercise_ids: Vec<String> = state.active_session.get_untracked()
        .map(|s| s.exercise_logs.iter().map(|e| e.exercise_id.clone()).collect())
        .unwrap_or_default();

    view! {
        <div class="page">
            <div class="page-header">
                <button class="back-btn" on:click=discard>"✕"</button>
                <div>
                    <div class="page-title">{day_name}</div>
                    <div class="text-muted text-sm">{date}</div>
                </div>
            </div>

            <For
                each=move || exercise_ids.clone()
                key=|id| id.clone()
                children=move |ex_id| view! { <ExerciseCard ex_id=ex_id open_ex=open_ex/> }
            />

            <button
                class="btn btn-finish btn-full"
                style="margin-top:8px"
                on:click=finish
            >
                {move || if all_done() { "✓  Finish Workout" } else { "Finish Workout" }}
            </button>
        </div>
    }
}

// ── Exercise card ─────────────────────────────────────────────────────────────

#[component]
fn ExerciseCard(
    ex_id: String,
    open_ex: RwSignal<Option<String>>,
) -> impl IntoView {
    let state = expect_context::<AppState>();

    let ex_name = {
        let ex_id = ex_id.clone();
        move || {
            state.active_session.get()
                .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())
                .map(|e| e.exercise_name)
                .unwrap_or_default()
        }
    };

    let target_info = {
        let ex_id = ex_id.clone();
        move || {
            state.active_session.get()
                .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())
                .map(|e| format!("{} sets × {}–{} reps", e.target_sets, e.reps_min, e.reps_max))
                .unwrap_or_default()
        }
    };

    let is_complete = {
        let ex_id = ex_id.clone();
        move || {
            state.active_session.get()
                .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())
                .map(|e| !e.sets.is_empty() && e.sets.iter().all(|s| s.completed))
                .unwrap_or(false)
        }
    };

    let is_expanded = {
        let ex_id = ex_id.clone();
        move || open_ex.get().as_deref() == Some(ex_id.as_str())
    };

    let toggle = {
        let ex_id = ex_id.clone();
        move |_| {
            open_ex.update(|opt| {
                if opt.as_deref() == Some(ex_id.as_str()) {
                    *opt = None;
                } else {
                    *opt = Some(ex_id.clone());
                }
            });
        }
    };

    // Computed once — same rationale as exercise_ids above: prevents inner For
    // from re-creating SetRow components (and re-firing CSS transitions) every
    // time a set is toggled.
    let set_indices: Vec<usize> = {
        let ex_id = ex_id.clone();
        state.active_session.get_untracked()
            .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())
            .map(|e| (0..e.sets.len()).collect())
            .unwrap_or_default()
    };

    let add_set = {
        let ex_id = ex_id.clone();
        move |_| {
            state.active_session.update(|opt| {
                if let Some(s) = opt.as_mut() {
                    if let Some(log) = s.exercise_logs.iter_mut().find(|e| e.exercise_id == ex_id) {
                        let last = log.sets.last().cloned().unwrap_or_default();
                        let n = log.sets.len() as u32 + 1;
                        log.sets.push(crate::models::SetLog {
                            set_number: n,
                            reps: last.reps,
                            weight_lbs: last.weight_lbs,
                            completed: false,
                        });
                    }
                }
            });
        }
    };

    let is_complete2 = is_complete.clone();
    let is_expanded2 = is_expanded.clone();

    view! {
        <div class="ex-card" class:ex-complete=is_complete>
            // Header — only the chevron toggles the accordion
            <div class="exercise-header">
                <div>
                    <div class="card-title">{ex_name}</div>
                    <div class="exercise-meta">{target_info}</div>
                </div>
                <div style="display:flex; align-items:center; gap:8px">
                    {move || is_complete2().then(|| view! {
                        <span class="exercise-complete-badge">"✓"</span>
                    })}
                    <span class="exercise-chevron" class:open=is_expanded on:click=toggle>"⌄"</span>
                </div>
            </div>

            // Animated accordion body (CSS grid trick — no JS height calc needed)
            <div class="exercise-body" class:open=is_expanded2>
                <div>
                    <div class="exercise-sets">
                        <For
                            each=move || set_indices.clone()
                            key=|i| *i
                            children={
                                let ex_id = ex_id.clone();
                                move |set_idx| {
                                    let ex_id = ex_id.clone();
                                    view! { <SetRow ex_id=ex_id set_idx=set_idx/> }
                                }
                            }
                        />
                        <button class="add-set-btn" on:click=add_set>"+ Add Set"</button>
                    </div>
                </div>
            </div>
        </div>
    }
}

// ── Set row ───────────────────────────────────────────────────────────────────

#[component]
fn SetRow(ex_id: String, set_idx: usize) -> impl IntoView {
    let state = expect_context::<AppState>();

    let weight = {
        let ex_id = ex_id.clone();
        move || {
            state.active_session.get()
                .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())
                .and_then(|e| e.sets.get(set_idx).cloned())
                .map(|s| s.weight_lbs)
                .unwrap_or(0.0)
        }
    };

    let reps = {
        let ex_id = ex_id.clone();
        move || {
            state.active_session.get()
                .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())
                .and_then(|e| e.sets.get(set_idx).cloned())
                .map(|s| s.reps)
                .unwrap_or(0)
        }
    };

    let is_done = {
        let ex_id = ex_id.clone();
        move || {
            state.active_session.get()
                .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())
                .and_then(|e| e.sets.get(set_idx).cloned())
                .map(|s| s.completed)
                .unwrap_or(false)
        }
    };

    let on_weight_change = {
        let ex_id = ex_id.clone();
        move |e| {
            let val: f32 = event_target_value(&e).parse().unwrap_or(0.0);
            state.active_session.update(|opt| {
                if let Some(s) = opt.as_mut() {
                    if let Some(log) = s.exercise_logs.iter_mut().find(|l| l.exercise_id == ex_id) {
                        if let Some(set) = log.sets.get_mut(set_idx) {
                            set.weight_lbs = val;
                        }
                    }
                }
            });
        }
    };

    let on_reps_change = {
        let ex_id = ex_id.clone();
        move |e| {
            let val: u32 = event_target_value(&e).parse().unwrap_or(0);
            state.active_session.update(|opt| {
                if let Some(s) = opt.as_mut() {
                    if let Some(log) = s.exercise_logs.iter_mut().find(|l| l.exercise_id == ex_id) {
                        if let Some(set) = log.sets.get_mut(set_idx) {
                            set.reps = val;
                        }
                    }
                }
            });
        }
    };

    let toggle_done = {
        let ex_id = ex_id.clone();
        move |_| {
            state.active_session.update(|opt| {
                if let Some(s) = opt.as_mut() {
                    if let Some(log) = s.exercise_logs.iter_mut().find(|e| e.exercise_id == ex_id) {
                        if let Some(set) = log.sets.get_mut(set_idx) {
                            set.completed = !set.completed;
                        }
                    }
                }
            });
        }
    };

    // Format weight: show integer if no fractional part
    let weight_str = move || {
        let w = weight();
        if w == 0.0 { String::new() }
        else if w.fract() == 0.0 { format!("{:.0}", w) }
        else { format!("{:.1}", w) }
    };

    let reps_str = move || {
        let r = reps();
        if r == 0 { String::new() } else { r.to_string() }
    };

    let is_done2 = is_done.clone();

    view! {
        <div class="set-row" class:set-done=is_done>
            <span class="set-num">"Set " {set_idx + 1}</span>

            <div class="set-inputs">
                <input
                    type="number"
                    inputmode="decimal"
                    step="2.5"
                    min="0"
                    class="set-num-input"
                    placeholder="wt"
                    prop:value=weight_str
                    on:change=on_weight_change
                />
                <span class="set-x">"×"</span>
                <input
                    type="number"
                    inputmode="numeric"
                    step="1"
                    min="0"
                    class="set-num-input"
                    placeholder="reps"
                    prop:value=reps_str
                    on:change=on_reps_change
                />
            </div>

            <button
                class="set-done-btn"
                class:done=is_done2
                on:click=toggle_done
            >
                "✓"
            </button>
        </div>
    }
}

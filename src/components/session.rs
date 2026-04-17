use leptos::prelude::*;

use crate::app::{AppState, View};

// ── Session view ──────────────────────────────────────────────────────────────

#[component]
pub fn SessionView(day_id: String) -> impl IntoView {
    let state = expect_context::<AppState>();

    move || match state.active_session.get() {
        None => view! {
            <div class="page">
                <p class="text-muted">"No active session."</p>
                <button class="btn btn-secondary mt-16"
                    on:click=move |_| state.navigate(View::Home)>
                    "← Back"
                </button>
            </div>
        }.into_any(),
        Some(_) => view! { <ActiveSession day_id=day_id.clone()/> }.into_any(),
    }
}

// ── Active session ────────────────────────────────────────────────────────────

#[component]
fn ActiveSession(day_id: String) -> impl IntoView {
    let state = expect_context::<AppState>();

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
            state.show_toast("Workout saved!");
            state.navigate(View::History);
        }
    };

    let discard = move |_| {
        state.active_session.set(None);
        state.navigate(View::Home);
    };

    let exercise_ids = move || {
        state.active_session.get()
            .map(|s| s.exercise_logs.iter().map(|e| e.exercise_id.clone()).collect::<Vec<_>>())
            .unwrap_or_default()
    };

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
                each=exercise_ids
                key=|id| id.clone()
                children=move |ex_id| view! { <ExerciseCard ex_id=ex_id/> }
            />

            <button
                class="btn btn-success btn-full"
                style="margin-top:8px"
                on:click=finish
            >
                {move || if all_done() { "✓ Finish Workout" } else { "Finish Workout" }}
            </button>
        </div>
    }
}

// ── Exercise card ─────────────────────────────────────────────────────────────

#[component]
fn ExerciseCard(ex_id: String) -> impl IntoView {
    let state = expect_context::<AppState>();
    let expanded = RwSignal::new(false);

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

    // Reactive set count — used by set_indices below
    let set_count = {
        let ex_id = ex_id.clone();
        move || {
            state.active_session.get()
                .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())
                .map(|e| e.sets.len())
                .unwrap_or(0)
        }
    };

    let set_indices = move || (0..set_count()).collect::<Vec<_>>();

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

    view! {
        <div class="card" style="margin-bottom:12px">
            // Header (tap to expand/collapse)
            <div class="exercise-header"
                on:click=move |_| expanded.update(|v| *v = !*v)>
                <div>
                    <div class="card-title">{ex_name}</div>
                    <div class="exercise-meta">{target_info}</div>
                </div>
                <div style="display:flex; align-items:center; gap:8px">
                    {move || is_complete().then(|| view! {
                        <span class="exercise-complete-badge">"✓ Done"</span>
                    })}
                    <span class="exercise-chevron" class:open=move || expanded.get()>"⌄"</span>
                </div>
            </div>

            // Sets panel — always in DOM, shown/hidden via CSS
            <div style:display=move || if expanded.get() { "block" } else { "none" }>
                <div class="exercise-sets">
                    <For
                        each=set_indices
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

    // Shared helper: update a set field
    let update_set = {
        let ex_id = ex_id.clone();
        move |f: &dyn Fn(&mut crate::models::SetLog)| {
            state.active_session.update(|opt| {
                if let Some(s) = opt.as_mut() {
                    if let Some(log) = s.exercise_logs.iter_mut().find(|e| e.exercise_id == ex_id) {
                        if let Some(set) = log.sets.get_mut(set_idx) {
                            f(set);
                        }
                    }
                }
            });
        }
    };

    let weight_dec = {
        let u = update_set.clone();
        move |_| u(&|s| s.weight_lbs = (s.weight_lbs - 2.5).max(0.0))
    };
    let weight_inc = {
        let u = update_set.clone();
        move |_| u(&|s| s.weight_lbs += 2.5)
    };
    let reps_dec = {
        let u = update_set.clone();
        move |_| u(&|s| s.reps = s.reps.saturating_sub(1))
    };
    let reps_inc = {
        let u = update_set.clone();
        move |_| u(&|s| s.reps += 1)
    };
    let toggle_done = {
        let u = update_set.clone();
        move |_| u(&|s| s.completed = !s.completed)
    };

    view! {
        <div class="set-row">
            <div class="set-num">{format!("Set {}", set_idx + 1)}</div>

            <div class="set-controls">
                // Weight stepper
                <div class="set-control">
                    <span class="stepper-label">"lbs"</span>
                    <div class="stepper">
                        <button class="stepper-btn" on:click=weight_dec>"−"</button>
                        <span class="stepper-val">{move || format!("{:.1}", weight())}</span>
                        <button class="stepper-btn" on:click=weight_inc>"+"</button>
                    </div>
                </div>

                // Reps stepper
                <div class="set-control">
                    <span class="stepper-label">"reps"</span>
                    <div class="stepper">
                        <button class="stepper-btn" on:click=reps_dec>"−"</button>
                        <span class="stepper-val">{move || reps().to_string()}</span>
                        <button class="stepper-btn" on:click=reps_inc>"+"</button>
                    </div>
                </div>
            </div>

            // Done checkmark
            <button
                class="set-done-btn"
                class:done=is_done
                on:click=toggle_done
            >
                "✓"
            </button>
        </div>
    }
}

use leptos::prelude::*;

use crate::app::{AppState, View};

// ── Session view ──────────────────────────────────────────────────────────────

#[component]
pub fn SessionView(_workout_id: String) -> impl IntoView {
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
                for log in &session.exercise_logs {
                    // Group sets by their check-off date (fallback: session start date).
                    let mut by_date: std::collections::BTreeMap<String, Vec<crate::models::SetLog>> =
                        std::collections::BTreeMap::new();
                    for set in &log.sets {
                        if let Some(date) = set.completed_date.clone() {
                            by_date.entry(date).or_default().push(set.clone());
                        }
                    }
                    for (date, sets) in by_date {
                        h.push(crate::models::ExerciseEntry {
                            id: uuid::Uuid::new_v4().to_string(),
                            date,
                            created_at: crate::app::current_datetime(),
                            exercise_name: log.exercise_name.clone(),
                            exercise_id: log.exercise_id.clone(),
                            session_id: Some(session.id.clone()),
                            day_id: Some(session.day_id.clone()),
                            day_name: Some(session.day_name.clone()),
                            target_sets: log.target_sets,
                            reps_min: log.reps_min,
                            reps_max: log.reps_max,
                            sets,
                            finalized: true,
                            target_duration_seconds: log.target_duration_seconds,
                        });
                    }
                }
            });
            state.active_session.set(None);
            state.show_toast("Workout saved! 💪");
            state.navigate(View::History);
        }
    };

    let discard = move |_| {
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
                .map(|e| {
                    if let Some(dur) = e.target_duration_seconds {
                        format!("{} sets × {}s hold", e.target_sets, dur)
                    } else {
                        let lib = state.library.get();
                        if crate::library::is_cardio_exercise(&e.exercise_id, &e.exercise_name, &lib) {
                            format!("{} min", e.reps_min)
                        } else {
                            format!("{} sets × {}–{} reps", e.target_sets, e.reps_min, e.reps_max)
                        }
                    }
                })
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

    let set_indices = {
        let ex_id = ex_id.clone();
        move || {
            state.active_session.get()
                .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())
                .map(|e| (0..e.sets.len()).collect::<Vec<_>>())
                .unwrap_or_default()
        }
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
                            weight: last.weight,
                            completed: false,
                            completed_date: None,
                            duration_seconds: last.duration_seconds,
                        });
                    }
                }
            });
        }
    };

    let nav_to_detail = {
        let ex_id = ex_id.clone();
        let workout_id = state.active_session.get_untracked()
            .map(|s| s.day_id.clone())
            .unwrap_or_default();
        move |_| {
            state.navigate(View::LibraryDetail {
                exercise_id: ex_id.clone(),
                from: Some(Box::new(View::Session { workout_id: workout_id.clone() })),
            });
        }
    };

    // Step-up nudge: if last session hit reps_max on every completed set,
    // suggest a +5 lb bump. Skipped for cardio (weight = RPE 1-10) and for
    // duration-based exercises.
    let step_up_target_weight = {
        let ex_id = ex_id.clone();
        move || -> Option<f32> {
            let log = state.active_session.get()
                .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())?;
            if log.target_duration_seconds.is_some() {
                return None;
            }
            let lib = state.library.get();
            if crate::library::is_cardio_exercise(&log.exercise_id, &log.exercise_name, &lib) {
                return None;
            }
            let history = state.history.get();
            let name_lc = log.exercise_name.to_lowercase();
            let prev = history.iter().rev().find(|e| {
                e.exercise_id == log.exercise_id || e.exercise_name.to_lowercase() == name_lc
            })?;
            let completed: Vec<_> = prev.sets.iter().filter(|s| s.completed).collect();
            if completed.is_empty() || !completed.iter().all(|s| s.reps >= log.reps_max) {
                return None;
            }
            let last_weight = completed.last().map(|s| s.weight).unwrap_or(0.0);
            if last_weight > 0.0 {
                Some(last_weight + 5.0)
            } else {
                None
            }
        }
    };

    let is_complete2 = is_complete.clone();
    let is_expanded2 = is_expanded.clone();

    view! {
        <div class="ex-card" class:ex-complete=is_complete>
            // Header — only the chevron toggles the accordion
            <div class="exercise-header">
                <div>
                    <div style="display:flex; align-items:baseline; gap:6px">
                        <div class="card-title">{ex_name}</div>
                        <button class="ex-info-btn" on:click=nav_to_detail>"ⓘ"</button>
                    </div>
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
                    {move || step_up_target_weight().map(|target| {
                        let label = if target.fract() == 0.0 {
                            format!("{:.0}", target)
                        } else {
                            format!("{}", target)
                        };
                        view! {
                            <div class="step-up-hint">
                                "💪 Hit top of range last time — try "
                                <strong>{label}" lb"</strong>
                            </div>
                        }
                    })}
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
        </div>
    }
}

// ── Set row ───────────────────────────────────────────────────────────────────

#[component]
fn SetRow(ex_id: String, set_idx: usize) -> impl IntoView {
    let state = expect_context::<AppState>();

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

    let duration = {
        let ex_id = ex_id.clone();
        move || {
            state.active_session.get()
                .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())
                .and_then(|e| e.sets.get(set_idx).cloned())
                .and_then(|s| s.duration_seconds)
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

    // Initial weight string from current state. Read untracked: we only need
    // the value once at component creation to seed the local input buffer.
    let initial_weight = state.active_session.get_untracked()
        .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())
        .and_then(|e| e.sets.get(set_idx).cloned())
        .map(|s| s.weight)
        .unwrap_or(0.0);
    let initial_weight_str = if initial_weight == 0.0 {
        String::new()
    } else if initial_weight.fract() == 0.0 {
        format!("{:.0}", initial_weight)
    } else {
        format!("{}", initial_weight)
    };
    // Local buffer for the weight/RPE input. Storing the user's raw typed
    // string here (rather than reformatting from the f32 every keystroke)
    // is what lets "12." pass through on the way to "12.5" — a reactive
    // formatter would strip the trailing dot before the next keypress.
    let weight_input: RwSignal<String> = RwSignal::new(initial_weight_str);

    let on_weight_change = {
        let ex_id = ex_id.clone();
        move |e| {
            let s = event_target_value(&e);
            weight_input.set(s.clone());
            let val: f32 = s.parse().unwrap_or(0.0);
            state.active_session.update(|opt| {
                if let Some(s) = opt.as_mut() {
                    if let Some(log) = s.exercise_logs.iter_mut().find(|l| l.exercise_id == ex_id) {
                        if let Some(set) = log.sets.get_mut(set_idx) {
                            set.weight = val;
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

    let on_duration_change = {
        let ex_id = ex_id.clone();
        move |e| {
            let val: u32 = event_target_value(&e).parse().unwrap_or(0);
            state.active_session.update(|opt| {
                if let Some(s) = opt.as_mut() {
                    if let Some(log) = s.exercise_logs.iter_mut().find(|l| l.exercise_id == ex_id) {
                        if let Some(set) = log.sets.get_mut(set_idx) {
                            set.duration_seconds = Some(val);
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
                            set.completed_date = if set.completed {
                                Some(crate::app::current_date())
                            } else {
                                None
                            };
                        }
                    }
                }
            });
        }
    };

    // Read the weight input's display string from the local buffer (set by
    // on_weight_change). Reading the raw text the user typed — instead of
    // reformatting from the parsed f32 — preserves trailing "." mid-typing.
    let weight_str = move || weight_input.get();

    let reps_str = move || {
        let r = reps();
        if r == 0 { String::new() } else { r.to_string() }
    };

    let is_cardio = {
        let ex_id = ex_id.clone();
        move || {
            let Some(session) = state.active_session.get() else { return false; };
            let Some(log) = session.exercise_logs.iter().find(|e| e.exercise_id == ex_id) else {
                return false;
            };
            crate::library::is_cardio_exercise(
                &log.exercise_id,
                &log.exercise_name,
                &state.library.get(),
            )
        }
    };

    let is_bodyweight = {
        let ex_id = ex_id.clone();
        move || {
            let Some(session) = state.active_session.get() else { return false; };
            let Some(log) = session.exercise_logs.iter().find(|e| e.exercise_id == ex_id) else {
                return false;
            };
            crate::library::is_bodyweight_exercise(
                &log.exercise_id,
                &log.exercise_name,
                &state.library.get(),
            )
        }
    };

    let duration_str = move || {
        let d = duration();
        if d == 0 { String::new() } else { d.to_string() }
    };

    let is_dur_static = {
        state.active_session.get_untracked()
            .and_then(|s| s.exercise_logs.iter().find(|e| e.exercise_id == ex_id).cloned())
            .and_then(|e| e.target_duration_seconds)
            .is_some()
    };

    let is_done2 = is_done.clone();

    if is_dur_static {
        view! {
            <div class="set-row" class:set-done=is_done>
                <span class="set-num">"Set " {set_idx + 1}</span>
                <div class="set-inputs">
                    <input
                        type="number"
                        inputmode="numeric"
                        step="5"
                        min="0"
                        class="set-num-input duration-input"
                        placeholder="sec"
                        prop:value=duration_str
                        on:change=on_duration_change
                    />
                    <span class="set-x">"s"</span>
                </div>
                <button class="set-done-btn" class:done=is_done2 on:click=toggle_done>"✓"</button>
            </div>
        }.into_any()
    } else {
        view! {
            <div class="set-row" class:set-done=is_done>
                <span class="set-num">"Set " {set_idx + 1}</span>

                <div class="set-inputs">
                    {
                        let is_cardio = is_cardio.clone();
                        let is_bodyweight = is_bodyweight.clone();
                        let weight_str = weight_str.clone();
                        let reps_str = reps_str.clone();
                        let on_weight_change = on_weight_change.clone();
                        let on_reps_change = on_reps_change.clone();
                        move || if is_cardio() {
                            // cardio precedence: minutes @ RPE
                            let reps_str = reps_str.clone();
                            let on_reps_change = on_reps_change.clone();
                            let weight_str = weight_str.clone();
                            let on_weight_change = on_weight_change.clone();
                            view! {
                                <input
                                    type="number"
                                    inputmode="numeric"
                                    step="1"
                                    min="0"
                                    class="set-num-input"
                                    placeholder="min"
                                    prop:value=reps_str
                                    on:input=on_reps_change
                                />
                                <span class="set-x">"@"</span>
                                <input
                                    type="number"
                                    inputmode="numeric"
                                    step="1"
                                    min="1"
                                    max="10"
                                    class="set-num-input"
                                    placeholder="RPE"
                                    prop:value=weight_str
                                    on:input=on_weight_change
                                />
                            }.into_any()
                        } else if is_bodyweight() {
                            // Bodyweight: only reps. No weight input, no separator.
                            // SetLog.weight stays at its default (0.0) and round-trips
                            // through serde unchanged, so existing data is untouched.
                            let reps_str = reps_str.clone();
                            let on_reps_change = on_reps_change.clone();
                            view! {
                                <input
                                    type="number"
                                    inputmode="numeric"
                                    step="1"
                                    min="0"
                                    class="set-num-input"
                                    placeholder="reps"
                                    prop:value=reps_str
                                    on:input=on_reps_change
                                />
                            }.into_any()
                        } else {
                            // Standard: weight × reps. type="text" + inputmode="decimal"
                            // preserves trailing "." during typing on mobile.
                            let weight_str = weight_str.clone();
                            let on_weight_change = on_weight_change.clone();
                            let reps_str = reps_str.clone();
                            let on_reps_change = on_reps_change.clone();
                            view! {
                                <input
                                    type="text"
                                    inputmode="decimal"
                                    pattern="[0-9]*\\.?[0-9]*"
                                    class="set-num-input"
                                    placeholder="wt"
                                    prop:value=weight_str
                                    on:input=on_weight_change
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
                                    on:input=on_reps_change
                                />
                            }.into_any()
                        }
                    }
                </div>
                <button class="set-done-btn" class:done=is_done2 on:click=toggle_done>"✓"</button>
            </div>
        }.into_any()
    }
}

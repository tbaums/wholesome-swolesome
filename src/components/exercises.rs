use leptos::prelude::*;

use crate::app::{AppState, View};
use crate::coach::CardioActuals;
use crate::models::{Exercise, ExerciseCategory, ExerciseEntry, LibraryExercise, SetLog};

// ── Freeform upsert helper ────────────────────────────────────────────────────

/// Finds today's active (non-finalized) freeform ExerciseEntry for this exercise,
/// or creates one pre-filled from the last completed set in history. Returns the index.
fn get_or_create_freeform(
    h: &mut Vec<ExerciseEntry>,
    exercise_name: &str,
    exercise_id: &str,
    target_sets: u32,
    reps_min: u32,
    reps_max: u32,
    today: &str,
) -> usize {
    if let Some(i) = h.iter().position(|e| {
        e.exercise_name == exercise_name && e.day_name.is_none() && e.date == today && !e.finalized
    }) {
        return i;
    }
    let (dw, dr) = h
        .iter()
        .rev()
        .filter(|e| e.exercise_name == exercise_name)
        .flat_map(|e| e.sets.iter())
        .filter(|s| s.completed)
        .next()
        .map(|s| (s.weight, s.reps))
        .unwrap_or((0.0, reps_min));
    let sets = (1..=target_sets)
        .map(|n| SetLog { set_number: n, reps: dr, weight: dw, completed: false, completed_date: None, duration_seconds: None, zone_minutes: None })
        .collect();
    h.push(ExerciseEntry {
        id: uuid::Uuid::new_v4().to_string(),
        date: today.to_string(),
        created_at: crate::app::current_datetime(),
        exercise_name: exercise_name.to_string(),
        exercise_id: exercise_id.to_string(),
        session_id: None,
        day_id: None,
        day_name: None,
        target_sets,
        reps_min,
        reps_max,
        sets,
        finalized: false,
        target_duration_seconds: None,
    });
    h.len() - 1
}

/// Write `actuals` (per-zone minutes from an Apple Health screenshot, parsed
/// by `crate::coach::parse_cardio_actuals`) into the freeform draft entry for
/// (exercise_name, selected_date). Creates the draft via [`get_or_create_freeform`]
/// if none exists. The last set is updated: `zone_minutes = Some(zones)`,
/// `reps = sum(zone minutes)` (so the displayed total minutes reflects the
/// import), `completed = true`, `completed_date = Some(selected_date)`. The
/// set's `weight` (RPE) is left alone — the screenshot doesn't carry RPE.
///
/// Returns the total minutes that were written (sum across zones).
#[allow(clippy::too_many_arguments)]
fn apply_cardio_actuals_to_freeform(
    h: &mut Vec<ExerciseEntry>,
    exercise_name: &str,
    exercise_id: &str,
    target_sets: u32,
    reps_min: u32,
    reps_max: u32,
    selected_date: &str,
    actuals: &CardioActuals,
) -> u32 {
    let i = get_or_create_freeform(
        h, exercise_name, exercise_id,
        target_sets, reps_min, reps_max, selected_date,
    );
    let total_minutes: u32 = actuals.zones.iter().map(|z| z.minutes).sum();
    let entry = &mut h[i];
    // Apply to the last set in the draft. If there are zero sets (shouldn't
    // happen because get_or_create_freeform always seeds target_sets ≥ 1),
    // push one.
    if entry.sets.is_empty() {
        entry.sets.push(SetLog {
            set_number: 1,
            ..SetLog::default()
        });
    }
    let last_idx = entry.sets.len() - 1;
    let last = &mut entry.sets[last_idx];
    last.reps = total_minutes;
    last.zone_minutes = Some(actuals.zones.clone());
    last.completed = true;
    last.completed_date = Some(selected_date.to_string());
    total_minutes
}

// ── Exercises view ────────────────────────────────────────────────────────────

#[component]
pub fn ExercisesView() -> impl IntoView {
    let state = expect_context::<AppState>();

    let open_ex: RwSignal<Option<String>> = RwSignal::new(None);

    // Popularity-sorted initial snapshot. Stored as a signal so newly created
    // exercises can be appended without re-sorting (and collapsing open accordions).
    // Sources: exercises seen in history, scheduled workouts, custom-added.
    let exercise_names: RwSignal<Vec<String>> = RwSignal::new({
        let scheduled = state.scheduled_workouts.get_untracked();
        let custom = state.custom_exercises.get_untracked();
        let history = state.history.get_untracked();

        let mut counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for entry in &history {
            *counts.entry(entry.exercise_name.clone()).or_insert(0) += 1;
        }

        let mut seen = std::collections::HashSet::new();
        let mut names = Vec::new();
        for entry in &history {
            if seen.insert(entry.exercise_name.clone()) {
                names.push(entry.exercise_name.clone());
            }
        }
        for w in &scheduled {
            for ex in &w.exercises {
                if seen.insert(ex.name.clone()) {
                    names.push(ex.name.clone());
                }
            }
        }
        for ex in &custom {
            if seen.insert(ex.name.clone()) {
                names.push(ex.name.clone());
            }
        }
        names.sort_by(|a, b| {
            let ca = counts.get(a).copied().unwrap_or(0);
            let cb = counts.get(b).copied().unwrap_or(0);
            cb.cmp(&ca)
        });
        names
    });

    let show_picker: RwSignal<bool> = RwSignal::new(false);
    let query: RwSignal<String> = RwSignal::new(String::new());

    // Add an exercise (from library or custom) to the freeform list and open its
    // accordion so the user can immediately log sets. The exercise is saved into
    // `custom_exercises` so it persists across reloads with its preferred id —
    // for library picks that id is the library id, which is what the heatmap
    // matches on.
    let add_exercise = move |name: String, id: String, target_sets: u32, reps_min: u32, reps_max: u32| {
        let name_for_lookup = name.clone();
        let already_saved = state.custom_exercises.with_untracked(|v| {
            v.iter().any(|e| e.name == name_for_lookup)
        });
        if !already_saved {
            state.custom_exercises.update(|v| v.push(Exercise {
                id,
                name: name.clone(),
                target_sets,
                reps_min,
                reps_max,
                category: ExerciseCategory::Main,
                notes: None,
            }));
        }
        exercise_names.update(|v| {
            if !v.iter().any(|n| n == &name) {
                v.insert(0, name.clone());
            }
        });
        open_ex.set(Some(name));
        query.set(String::new());
        show_picker.set(false);
    };

    let filtered_library = move || -> Vec<LibraryExercise> {
        let lib = state.library.get();
        let q = query.get().trim().to_lowercase();
        if q.is_empty() {
            lib.into_iter().take(40).collect()
        } else {
            let mut starts: Vec<LibraryExercise> = Vec::new();
            let mut contains: Vec<LibraryExercise> = Vec::new();
            for e in lib {
                let n = e.name.to_lowercase();
                if n.starts_with(&q) {
                    starts.push(e);
                } else if n.contains(&q) {
                    contains.push(e);
                }
                if starts.len() + contains.len() >= 80 { break; }
            }
            starts.into_iter().chain(contains.into_iter()).take(40).collect()
        }
    };

    let cancel = move |_| {
        query.set(String::new());
        show_picker.set(false);
    };

    // Date to log for — defaults to today, user can backdate if they forgot to
    // log yesterday's session in the moment. Drives both the entry's `date`
    // field and each set's `completed_date`. Capped at today (no future dates).
    let selected_date: RwSignal<String> = RwSignal::new(crate::app::current_date());
    let today_str = crate::app::current_date();

    view! {
        <div class="page">
            <div class="page-header">
                <h1 class="page-title">"Exercises"</h1>
            </div>

            <div class="card" style="margin-bottom:12px; display:flex; align-items:center; gap:10px; flex-wrap:wrap">
                <label class="text-sm text-muted" style="flex-shrink:0">"Logging for"</label>
                <input
                    type="date"
                    class="input"
                    style="flex:1; min-width:140px; max-width:200px"
                    max=today_str.clone()
                    prop:value=move || selected_date.get()
                    on:change=move |e| {
                        let v = event_target_value(&e);
                        // Empty value (user cleared the field) falls back to today.
                        if v.trim().is_empty() {
                            selected_date.set(crate::app::current_date());
                        } else {
                            selected_date.set(v);
                        }
                    }
                />
                {
                    let today_str = today_str.clone();
                    move || if selected_date.get() != today_str {
                        view! {
                            <span
                                class="text-sm"
                                style="color:var(--accent); font-weight:600"
                            >"⏱ Backdated"</span>
                        }.into_any()
                    } else {
                        ().into_any()
                    }
                }
            </div>

            {move || if show_picker.get() {
                view! {
                    <div class="new-exercise-form card" style="margin-bottom:12px">
                        <input
                            type="text"
                            placeholder="Filter exercises…"
                            class="new-exercise-search"
                            autofocus
                            prop:value=move || query.get()
                            on:input=move |e| query.set(event_target_value(&e))
                        />
                        <div class="new-exercise-results">
                            <For
                                each=filtered_library
                                key=|e| e.id.clone()
                                children=move |e| {
                                    let name = e.name.clone();
                                    let id = e.id.clone();
                                    let (s, mn, mx) = defaults_for_category(&e.category);
                                    let primary = e.primary_muscles
                                        .first()
                                        .map(|m| crate::models::muscle_label(m).to_string())
                                        .unwrap_or_default();
                                    let equip = e.equipment.clone().unwrap_or_default();
                                    let meta = match (primary.is_empty(), equip.is_empty()) {
                                        (true, true) => String::new(),
                                        (true, false) => equip,
                                        (false, true) => primary,
                                        (false, false) => format!("{} · {}", primary, equip),
                                    };
                                    let on_click = move |_| {
                                        add_exercise(name.clone(), id.clone(), s, mn, mx);
                                    };
                                    view! {
                                        <div class="library-item new-exercise-result" on:click=on_click>
                                            <div>
                                                <div class="library-item-name">{e.name.clone()}</div>
                                                <div class="library-item-meta">{meta}</div>
                                            </div>
                                        </div>
                                    }
                                }
                            />
                            {move || {
                                let q = query.get().trim().to_string();
                                if q.is_empty() { return ().into_any(); }
                                let lib = state.library.get();
                                let q_lc = q.to_lowercase();
                                if lib.iter().any(|e| e.name.to_lowercase() == q_lc) {
                                    return ().into_any();
                                }
                                let label = q.clone();
                                let on_click = move |_| {
                                    let id = uuid::Uuid::new_v4().to_string();
                                    add_exercise(q.clone(), id, 3, 8, 12);
                                };
                                view! {
                                    <div class="library-item new-exercise-custom" on:click=on_click>
                                        <div>
                                            <div class="library-item-name">
                                                {format!("+ Use \"{}\" as custom", label)}
                                            </div>
                                            <div class="library-item-meta">
                                                "Not in library — won't color the heatmap"
                                            </div>
                                        </div>
                                    </div>
                                }.into_any()
                            }}
                        </div>
                        <button
                            class="btn btn-secondary btn-full"
                            style="margin-top:8px"
                            on:click=cancel
                        >"Cancel"</button>
                    </div>
                }.into_any()
            } else {
                view! {
                    <button
                        class="btn btn-secondary btn-full new-exercise-btn"
                        style="margin-bottom:12px"
                        on:click=move |_| show_picker.set(true)
                    >"+ New Exercise"</button>
                }.into_any()
            }}

            <For
                each=move || exercise_names.get()
                key=|name| name.clone()
                children=move |exercise_name| {
                    view! { <ExerciseFreeformCard exercise_name=exercise_name open_ex=open_ex selected_date=selected_date/> }
                }
            />
        </div>
    }
}

fn defaults_for_category(category: &str) -> (u32, u32, u32) {
    match category {
        // Cardio: one block, default 20 minutes (stored in reps_min/max).
        "cardio" => (1, 20, 20),
        "stretching" => (1, 1, 1),
        _ => (3, 8, 12),
    }
}

// ── Exercise freeform card ────────────────────────────────────────────────────

#[component]
fn ExerciseFreeformCard(
    exercise_name: String,
    open_ex: RwSignal<Option<String>>,
    /// Date to log this exercise for. Default is today, but the user can backdate
    /// from the picker at the top of the Exercises page.
    selected_date: RwSignal<String>,
) -> impl IntoView {
    let state = expect_context::<AppState>();

    // Look up metadata: scheduled workouts first (most recent prescription), then customs, then history.
    let scheduled = state.scheduled_workouts.get_untracked();
    let custom = state.custom_exercises.get_untracked();
    let history = state.history.get_untracked();
    let (exercise_id, target_sets, reps_min, reps_max) = scheduled
        .iter()
        .rev()
        .flat_map(|w| w.exercises.iter())
        .find(|e| e.name == exercise_name)
        .map(|e| (
            e.library_id.clone().unwrap_or_else(|| e.name.clone()),
            e.target_sets,
            e.reps_min,
            e.reps_max,
        ))
        .or_else(|| custom.iter().find(|e| e.name == exercise_name).map(|e| {
            (e.id.clone(), e.target_sets, e.reps_min, e.reps_max)
        }))
        .or_else(|| history.iter().rev().find(|e| e.exercise_name == exercise_name).map(|e| {
            (e.exercise_id.clone(), e.target_sets, e.reps_min, e.reps_max)
        }))
        .unwrap_or_else(|| (String::new(), 3, 8, 12));

    let is_cardio = {
        let exercise_id = exercise_id.clone();
        let exercise_name = exercise_name.clone();
        move || crate::library::is_cardio_exercise(
            &exercise_id,
            &exercise_name,
            &state.library.get(),
        )
    };

    let meta = {
        let is_cardio = is_cardio.clone();
        move || {
            if is_cardio() {
                format!("{} min", reps_min)
            } else {
                format!("{} sets × {}–{} reps", target_sets, reps_min, reps_max)
            }
        }
    };

    let is_expanded = {
        let exercise_name = exercise_name.clone();
        move || open_ex.get().as_deref() == Some(exercise_name.as_str())
    };

    let toggle = {
        let exercise_name = exercise_name.clone();
        move |_| {
            open_ex.update(|opt| {
                if opt.as_deref() == Some(exercise_name.as_str()) {
                    *opt = None;
                } else {
                    *opt = Some(exercise_name.clone());
                }
            });
        }
    };

    // set_indices is reactive: grows when the user adds a set OR when the
    // user changes selected_date (a different date's draft has different sets).
    // Skips finalized entries — those are closed history records, not the active session.
    let set_indices = {
        let exercise_name = exercise_name.clone();
        move || {
            let active_date = selected_date.get();
            state.history
                .get()
                .iter()
                .find(|e| e.exercise_name == exercise_name && e.day_name.is_none() && e.date == active_date && !e.finalized)
                .map(|e| (0..e.sets.len()).collect::<Vec<_>>())
                .unwrap_or_else(|| (0..target_sets as usize).collect())
        }
    };

    let add_set = {
        let exercise_name = exercise_name.clone();
        let exercise_id = exercise_id.clone();
        move |_| {
            let active_date = selected_date.get_untracked();
            state.history.update(|h| {
                let i = get_or_create_freeform(
                    h, &exercise_name, &exercise_id,
                    target_sets, reps_min, reps_max, &active_date,
                );
                let last = h[i].sets.last().cloned().unwrap_or_default();
                let n = h[i].sets.len() as u32 + 1;
                h[i].sets.push(SetLog {
                    set_number: n,
                    reps: last.reps,
                    weight: last.weight,
                    completed: false,
                    completed_date: None,
                    duration_seconds: None,
                    zone_minutes: None,                });
            });
        }
    };

    // Pending state: true during the 2-second window after the complete button is clicked.
    let is_pending: RwSignal<bool> = RwSignal::new(false);

    // Single complete button handler: save checked sets (finalize entry), close accordion after 2s.
    let on_complete = {
        let exercise_name = exercise_name.clone();
        move |_| {
            if is_pending.get_untracked() { return; }
            is_pending.set(true);
            // Snapshot the active date NOW so a date change mid-debounce
            // can't redirect the finalize to a different draft.
            let active_date = selected_date.get_untracked();
            let exercise_name = exercise_name.clone();
            let cb = wasm_bindgen::closure::Closure::once(move || {
                state.history.update(|h| {
                    if let Some(i) = h.iter().position(|e| {
                        e.exercise_name == exercise_name
                            && e.day_name.is_none()
                            && e.date == active_date
                            && !e.finalized
                    }) {
                        let has_completed = h[i].sets.iter().any(|s| s.completed);
                        if has_completed {
                            h[i].sets.retain(|s| s.completed);
                            h[i].finalized = true;
                        } else {
                            h.remove(i);
                        }
                    }
                });
                open_ex.set(None);
                is_pending.set(false);
            });
            if let Some(window) = web_sys::window() {
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    wasm_bindgen::JsCast::unchecked_ref::<js_sys::Function>(cb.as_ref()),
                    2000,
                );
            }
            cb.forget();
        }
    };

    let nav_to_detail = {
        let exercise_id = exercise_id.clone();
        move |_| {
            state.navigate(View::LibraryDetail {
                exercise_id: exercise_id.clone(),
                from: Some(Box::new(View::Exercises)),
            });
        }
    };

    let is_expanded2 = is_expanded.clone();

    view! {
        <div class="ex-card">
            <div class="exercise-header">
                <div>
                    <div style="display:flex; align-items:baseline; gap:6px">
                        <div class="card-title">{exercise_name.clone()}</div>
                        <button class="ex-info-btn" on:click=nav_to_detail>"ⓘ"</button>
                    </div>
                    <div class="exercise-meta">{meta}</div>
                </div>
                <div style="display:flex; align-items:center; gap:8px">
                    <button
                        class="ex-complete-btn"
                        class:ex-complete-pending=is_pending
                        style="-webkit-tap-highlight-color:transparent"
                        on:click=on_complete
                    >"✓"</button>
                    <span class="exercise-chevron" class:open=is_expanded on:click=toggle>"⌄"</span>
                </div>
            </div>
            <div class="exercise-body" class:open=is_expanded2>
                <div>
                    <div class="exercise-sets">
                        <For
                            each=set_indices
                            key=|i| *i
                            children={
                                let exercise_name = exercise_name.clone();
                                let exercise_id = exercise_id.clone();
                                move |set_idx| {
                                    let exercise_name = exercise_name.clone();
                                    let exercise_id = exercise_id.clone();
                                    view! {
                                        <FreeformSetRow
                                            exercise_name=exercise_name
                                            exercise_id=exercise_id
                                            target_sets=target_sets
                                            reps_min=reps_min
                                            reps_max=reps_max
                                            set_idx=set_idx
                                            selected_date=selected_date
                                        />
                                    }
                                }
                            }
                        />
                        <button class="add-set-btn" on:click=add_set>"+ Add Set"</button>
                    </div>

                    // Cardio-only: Apple Health screenshot → JSON → zone_minutes flow.
                    // Mirrors the session.rs CardioActualsImport but scoped to this
                    // freeform exercise (the prompt embeds the specific library_id and
                    // the import writes to the draft entry for selected_date).
                    {
                        let is_cardio = is_cardio.clone();
                        let exercise_name = exercise_name.clone();
                        let exercise_id = exercise_id.clone();
                        move || is_cardio().then(|| {
                            view! {
                                <FreeformCardioImportCard
                                    exercise_name=exercise_name.clone()
                                    exercise_id=exercise_id.clone()
                                    target_sets=target_sets
                                    reps_min=reps_min
                                    reps_max=reps_max
                                    selected_date=selected_date
                                />
                            }
                        })
                    }
                </div>
            </div>
        </div>
    }
}

// ── Freeform cardio screenshot-import card ────────────────────────────────────
//
// Per-exercise sibling of session.rs::CardioActualsImport. Renders inside the
// accordion body of an ad-hoc cardio exercise. Same prompt shape and parser as
// the session version, so the off-app Claude conversation is interchangeable.

#[component]
fn FreeformCardioImportCard(
    exercise_name: String,
    exercise_id: String,
    target_sets: u32,
    reps_min: u32,
    reps_max: u32,
    selected_date: RwSignal<String>,
) -> impl IntoView {
    let state = expect_context::<AppState>();
    let response_text: RwSignal<String> = RwSignal::new(String::new());
    let status: RwSignal<Option<String>> = RwSignal::new(None);

    let prompt_text = {
        let exercise_name = exercise_name.clone();
        let exercise_id = exercise_id.clone();
        move || -> String {
            // exercise_id should always be a real library id for cardio (added via
            // picker), but if for any reason it isn't, fall back to the name.
            let id_token = if exercise_id.is_empty() {
                exercise_name.as_str()
            } else {
                exercise_id.as_str()
            };
            format!(
                "Here's my Apple Health workout-summary screenshot. Return the per-zone heart-rate minutes \
                 from it as a single fenced ```json code block, nothing else:\n\
                 \n\
                 ```json\n\
                 {{\"cardio_actuals\":{{\"exercise_id\":\"{id_token}\",\"zones\":[{{\"zone\":1,\"minutes\":<N>}},{{\"zone\":2,\"minutes\":<N>}},{{\"zone\":3,\"minutes\":<N>}},{{\"zone\":4,\"minutes\":<N>}},{{\"zone\":5,\"minutes\":<N>}}]}}}}\n\
                 ```\n\
                 \n\
                 Use Apple's zone numbering 1–5. Omit any zone with 0 minutes. The exercise is \"{exercise_name}\"; use that exact library_id."
            )
        }
    };

    let copy_prompt = {
        let prompt_text = prompt_text.clone();
        move |_| {
            let text = prompt_text();
            if let Some(window) = web_sys::window() {
                let _ = window.navigator().clipboard().write_text(&text);
                state.show_toast("Prompt copied — paste into Claude with your screenshot");
            }
        }
    };

    let import = {
        let exercise_name = exercise_name.clone();
        let exercise_id = exercise_id.clone();
        move |_| {
            let text = response_text.get_untracked();
            if text.trim().is_empty() {
                status.set(Some("Paste Claude's JSON response first.".into()));
                return;
            }
            match crate::coach::parse_cardio_actuals(&text) {
                Ok(actuals) => {
                    let active_date = selected_date.get_untracked();
                    let mut total: u32 = 0;
                    state.history.update(|h| {
                        total = apply_cardio_actuals_to_freeform(
                            h, &exercise_name, &exercise_id,
                            target_sets, reps_min, reps_max,
                            &active_date, &actuals,
                        );
                    });
                    status.set(Some(format!(
                        "✓ {} min imported across {} zone(s)",
                        total,
                        actuals.zones.len(),
                    )));
                    response_text.set(String::new());
                    state.show_toast("Cardio zones imported");
                }
                Err(e) => status.set(Some(format!("✗ {e}"))),
            }
        }
    };

    view! {
        <div class="cardio-import-card" style="margin-top:10px">
            <div class="ci-title">"Paste Apple Health cardio summary"</div>
            <div class="ci-blurb">
                "1. Copy the prompt. 2. Paste it into a Claude conversation with your Apple Health workout-summary screenshot. 3. Paste Claude's JSON response below."
            </div>
            <div class="ci-prompt-wrap">
                <pre class="ci-prompt">{prompt_text}</pre>
                <button class="ci-copy-btn" on:click=copy_prompt aria-label="Copy prompt">"📋 Copy"</button>
            </div>
            <textarea
                placeholder="{ &quot;cardio_actuals&quot;: { ... } }"
                prop:value=move || response_text.get()
                on:input=move |e| response_text.set(event_target_value(&e))
            />
            {move || status.get().map(|s| view! {
                <div class="text-sm" style="margin-top:6px">{s}</div>
            })}
            <button
                class="btn btn-primary btn-full"
                style="margin-top:8px"
                on:click=import
            >"Import zone minutes"</button>
        </div>
    }
}

// ── Freeform set row ──────────────────────────────────────────────────────────

#[component]
fn FreeformSetRow(
    exercise_name: String,
    exercise_id: String,
    target_sets: u32,
    reps_min: u32,
    reps_max: u32,
    set_idx: usize,
    /// Page-level "logging for" date. Drives both the entry's `date` field and
    /// each set's `completed_date`. The same signal is shared by every set row.
    selected_date: RwSignal<String>,
) -> impl IntoView {
    let state = expect_context::<AppState>();

    // ── reactive readers ──────────────────────────────────────────────────────

    let weight = {
        let exercise_name = exercise_name.clone();
        move || -> f32 {
            let active_date = selected_date.get();
            let history = state.history.get();
            if let Some(entry) = history.iter().find(|e| {
                e.exercise_name == exercise_name && e.day_name.is_none() && e.date == active_date && !e.finalized
            }) {
                return entry.sets.get(set_idx).map(|s| s.weight).unwrap_or(0.0);
            }
            history.iter().rev()
                .filter(|e| e.exercise_name == exercise_name)
                .flat_map(|e| e.sets.iter())
                .filter(|s| s.completed)
                .next()
                .map(|s| s.weight)
                .unwrap_or(0.0)
        }
    };

    let reps = {
        let exercise_name = exercise_name.clone();
        move || -> u32 {
            let active_date = selected_date.get();
            let history = state.history.get();
            if let Some(entry) = history.iter().find(|e| {
                e.exercise_name == exercise_name && e.day_name.is_none() && e.date == active_date && !e.finalized
            }) {
                return entry.sets.get(set_idx).map(|s| s.reps).unwrap_or(reps_min);
            }
            history.iter().rev()
                .filter(|e| e.exercise_name == exercise_name)
                .flat_map(|e| e.sets.iter())
                .filter(|s| s.completed)
                .next()
                .map(|s| s.reps)
                .unwrap_or(reps_min)
        }
    };

    let is_done = {
        let exercise_name = exercise_name.clone();
        move || {
            let active_date = selected_date.get();
            state.history.get()
                .iter()
                .find(|e| e.exercise_name == exercise_name && e.day_name.is_none() && e.date == active_date && !e.finalized)
                .and_then(|e| e.sets.get(set_idx))
                .map(|s| s.completed)
                .unwrap_or(false)
        }
    };

    // ── event handlers ────────────────────────────────────────────────────────

    let on_weight_change = {
        let exercise_name = exercise_name.clone();
        let exercise_id = exercise_id.clone();
        move |e| {
            let val: f32 = event_target_value(&e).parse().unwrap_or(0.0);
            let active_date = selected_date.get_untracked();
            state.history.update(|h| {
                let i = get_or_create_freeform(
                    h, &exercise_name, &exercise_id,
                    target_sets, reps_min, reps_max, &active_date,
                );
                if let Some(set) = h[i].sets.get_mut(set_idx) {
                    set.weight = val;
                }
            });
        }
    };

    let on_reps_change = {
        let exercise_name = exercise_name.clone();
        let exercise_id = exercise_id.clone();
        move |e| {
            let val: u32 = event_target_value(&e).parse().unwrap_or(0);
            let active_date = selected_date.get_untracked();
            state.history.update(|h| {
                let i = get_or_create_freeform(
                    h, &exercise_name, &exercise_id,
                    target_sets, reps_min, reps_max, &active_date,
                );
                if let Some(set) = h[i].sets.get_mut(set_idx) {
                    set.reps = val;
                }
            });
        }
    };

    let toggle_done = {
        let exercise_name = exercise_name.clone();
        let exercise_id = exercise_id.clone();
        move |_| {
            let active_date = selected_date.get_untracked();
            state.history.update(|h| {
                let i = get_or_create_freeform(
                    h, &exercise_name, &exercise_id,
                    target_sets, reps_min, reps_max, &active_date,
                );
                if let Some(set) = h[i].sets.get_mut(set_idx) {
                    set.completed = !set.completed;
                    set.completed_date = if set.completed {
                        Some(active_date.clone())
                    } else {
                        None
                    };
                }
            });
        }
    };

    // ── derived display values ────────────────────────────────────────────────

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

    let is_cardio = {
        let exercise_id = exercise_id.clone();
        let exercise_name = exercise_name.clone();
        move || crate::library::is_cardio_exercise(
            &exercise_id,
            &exercise_name,
            &state.library.get(),
        )
    };

    let is_bodyweight = {
        let exercise_id = exercise_id.clone();
        let exercise_name = exercise_name.clone();
        move || crate::library::is_bodyweight_exercise(
            &exercise_id,
            &exercise_name,
            &state.library.get(),
        )
    };

    let is_done2 = is_done.clone();

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
                                on:change=on_reps_change
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
                                on:change=on_weight_change
                            />
                        }.into_any()
                    } else if is_bodyweight() {
                        // Bodyweight: only reps. weight stays at 0 in storage, schema unchanged.
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
                                on:change=on_reps_change
                            />
                        }.into_any()
                    } else {
                        let weight_str = weight_str.clone();
                        let on_weight_change = on_weight_change.clone();
                        let reps_str = reps_str.clone();
                        let on_reps_change = on_reps_change.clone();
                        view! {
                            <input
                                type="number"
                                inputmode="decimal"
                                step="any"
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
                        }.into_any()
                    }
                }
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

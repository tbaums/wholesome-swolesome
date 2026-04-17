use leptos::prelude::*;

use crate::app::{AppState, View};
use crate::csv_utils::{download_file, export_plan_csv, import_plan_csv};
use crate::models::{Exercise, ExerciseCategory, WorkoutDay};

// ── Plan editor (day list) ────────────────────────────────────────────────────

#[component]
pub fn PlanEditorView() -> impl IntoView {
    let state = expect_context::<AppState>();

    let add_day = move |_| {
        state.plan.update(|p| {
            let n = p.days.len() + 1;
            p.days.push(WorkoutDay {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("Day {}", n),
                exercises: vec![],
            });
        });
    };

    view! {
        <div class="page">
            <div class="page-header">
                <h1 class="page-title">"Workout Plan"</h1>
                <button
                    class="btn btn-secondary btn-sm"
                    style="margin-left:auto"
                    on:click=move |_| state.navigate(View::ImportExport)
                >
                    "Import / Export"
                </button>
            </div>

            <For
                each=move || state.plan.get().days
                key=|day| day.id.clone()
                children=move |day| {
                    let day_id = day.id.clone();
                    let ex_count = day.exercises.len();
                    view! {
                        <div
                            class="card day-item"
                            style="margin-bottom:8px; cursor:pointer"
                            on:click=move |_| state.navigate(View::DayEditor { day_id: day_id.clone() })
                        >
                            <div>
                                <div class="card-title">{day.name}</div>
                                <div class="card-sub">{ex_count} " exercises"</div>
                            </div>
                            <span style="color:var(--text-muted)">"›"</span>
                        </div>
                    }
                }
            />

            <button class="btn btn-secondary btn-full" on:click=add_day>
                "+ Add Day"
            </button>
        </div>
    }
}

// ── Day editor ────────────────────────────────────────────────────────────────

#[component]
pub fn DayEditorView(day_id: String) -> impl IntoView {
    let state = expect_context::<AppState>();

    let day = {
        let day_id = day_id.clone();
        move || {
            state.plan.get().days.into_iter().find(|d| d.id == day_id)
        }
    };

    // Editable day name
    let name_input = RwSignal::new(
        day().map(|d| d.name.clone()).unwrap_or_default()
    );

    let save_name = {
        let day_id = day_id.clone();
        move |_| {
            let new_name = name_input.get();
            state.plan.update(|p| {
                if let Some(d) = p.days.iter_mut().find(|d| d.id == day_id) {
                    d.name = new_name.clone();
                }
            });
        }
    };

    let delete_day = {
        let day_id = day_id.clone();
        move |_| {
            state.plan.update(|p| p.days.retain(|d| d.id != day_id));
            state.navigate(View::PlanEditor);
        }
    };

    let add_exercise = {
        let day_id = day_id.clone();
        move |_| {
            state.plan.update(|p| {
                if let Some(d) = p.days.iter_mut().find(|d| d.id == day_id) {
                    d.exercises.push(Exercise {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: "New Exercise".into(),
                        target_sets: 3,
                        reps_min: 8,
                        reps_max: 12,
                        category: ExerciseCategory::Main,
                        notes: None,
                    });
                }
            });
        }
    };

    let exercise_ids = {
        let day_id = day_id.clone();
        move || {
            state.plan.get()
                .days.into_iter()
                .find(|d| d.id == day_id)
                .map(|d| d.exercises.iter().map(|e| e.id.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        }
    };

    view! {
        <div class="page">
            <div class="page-header">
                <button class="back-btn" on:click=move |_| state.navigate(View::PlanEditor)>
                    "‹ Back"
                </button>
                <h1 class="page-title">"Edit Day"</h1>
            </div>

            <div class="card">
                <div class="form-group">
                    <label>"Day name"</label>
                    <input
                        type="text"
                        prop:value=move || name_input.get()
                        on:input=move |e| name_input.set(event_target_value(&e))
                        on:blur=save_name.clone()
                    />
                </div>
            </div>

            <div class="card-title" style="margin: 16px 0 8px">"Exercises"</div>

            <For
                each=exercise_ids
                key=|id| id.clone()
                children={
                    let day_id = day_id.clone();
                    move |ex_id| {
                        view! { <ExerciseEditCard day_id=day_id.clone() ex_id=ex_id/> }
                    }
                }
            />

            <button class="btn btn-secondary btn-full" on:click=add_exercise>
                "+ Add Exercise"
            </button>

            <div class="divider"/>

            <button class="btn btn-danger btn-full" on:click=delete_day>
                "Delete Day"
            </button>
        </div>
    }
}

// ── Exercise edit card ────────────────────────────────────────────────────────

#[component]
fn ExerciseEditCard(day_id: String, ex_id: String) -> impl IntoView {
    let state = expect_context::<AppState>();
    let expanded = RwSignal::new(false);

    let get_ex = {
        let day_id = day_id.clone();
        let ex_id = ex_id.clone();
        move || {
            state.plan.get()
                .days.into_iter()
                .find(|d| d.id == day_id)
                .and_then(|d| d.exercises.into_iter().find(|e| e.id == ex_id))
        }
    };

    let name_val = RwSignal::new(get_ex().map(|e| e.name.clone()).unwrap_or_default());
    let sets_val = RwSignal::new(get_ex().map(|e| e.target_sets.to_string()).unwrap_or_else(|| "3".into()));
    let reps_min_val = RwSignal::new(get_ex().map(|e| e.reps_min.to_string()).unwrap_or_else(|| "8".into()));
    let reps_max_val = RwSignal::new(get_ex().map(|e| e.reps_max.to_string()).unwrap_or_else(|| "12".into()));
    let notes_val = RwSignal::new(get_ex().and_then(|e| e.notes.clone()).unwrap_or_default());
    let cat_val = RwSignal::new(get_ex().map(|e| e.category.label().to_string()).unwrap_or_else(|| "Main".into()));

    let save = {
        let day_id = day_id.clone();
        let ex_id = ex_id.clone();
        move || {
            let name = name_val.get();
            let sets: u32 = sets_val.get().parse().unwrap_or(3);
            let reps_min: u32 = reps_min_val.get().parse().unwrap_or(8);
            let reps_max: u32 = reps_max_val.get().parse().unwrap_or(12).max(reps_min);
            let notes = notes_val.get();
            let cat = match cat_val.get().as_str() {
                "Core" => ExerciseCategory::Core,
                "Cardio" => ExerciseCategory::Cardio,
                _ => ExerciseCategory::Main,
            };

            state.plan.update(|p| {
                if let Some(d) = p.days.iter_mut().find(|d| d.id == day_id) {
                    if let Some(e) = d.exercises.iter_mut().find(|e| e.id == ex_id) {
                        e.name = name;
                        e.target_sets = sets;
                        e.reps_min = reps_min;
                        e.reps_max = reps_max;
                        e.notes = if notes.is_empty() { None } else { Some(notes) };
                        e.category = cat;
                    }
                }
            });
        }
    };

    let delete_ex = {
        let day_id = day_id.clone();
        let ex_id = ex_id.clone();
        move |_| {
            state.plan.update(|p| {
                if let Some(d) = p.days.iter_mut().find(|d| d.id == day_id) {
                    d.exercises.retain(|e| e.id != ex_id);
                }
            });
        }
    };

    view! {
        <div class="card" style="margin-bottom:8px">
            <div class="exercise-header" on:click=move |_| expanded.update(|v| *v = !*v)>
                <div>
                    <div class="card-title">{move || name_val.get()}</div>
                    <div class="exercise-meta">
                        {move || format!("{} sets × {}–{} reps", sets_val.get(), reps_min_val.get(), reps_max_val.get())}
                    </div>
                </div>
                <span class="exercise-chevron" class:open=move || expanded.get()>"⌄"</span>
            </div>

            {move || expanded.get().then({
                let save = save.clone();
                let delete_ex = delete_ex.clone();
                move || view! {
                    <div style="margin-top:14px">
                        <div class="form-group">
                            <label>"Name"</label>
                            <input type="text"
                                prop:value=move || name_val.get()
                                on:input=move |e| name_val.set(event_target_value(&e))
                                on:blur={let s = save.clone(); move |_| s()}
                            />
                        </div>

                        <div class="row">
                            <div class="form-group flex-1">
                                <label>"Sets"</label>
                                <input type="number" min="1" max="20"
                                    prop:value=move || sets_val.get()
                                    on:input=move |e| sets_val.set(event_target_value(&e))
                                    on:blur={let s = save.clone(); move |_| s()}
                                />
                            </div>
                            <div class="form-group flex-1">
                                <label>"Reps min"</label>
                                <input type="number" min="1" max="100"
                                    prop:value=move || reps_min_val.get()
                                    on:input=move |e| reps_min_val.set(event_target_value(&e))
                                    on:blur={let s = save.clone(); move |_| s()}
                                />
                            </div>
                            <div class="form-group flex-1">
                                <label>"Reps max"</label>
                                <input type="number" min="1" max="100"
                                    prop:value=move || reps_max_val.get()
                                    on:input=move |e| reps_max_val.set(event_target_value(&e))
                                    on:blur={let s = save.clone(); move |_| s()}
                                />
                            </div>
                        </div>

                        <div class="form-group">
                            <label>"Category"</label>
                            <select
                                prop:value=move || cat_val.get()
                                on:change={
                                    let s = save.clone();
                                    move |e| { cat_val.set(event_target_value(&e)); s(); }
                                }
                            >
                                <option value="Main">"Main"</option>
                                <option value="Core">"Core"</option>
                                <option value="Cardio">"Cardio"</option>
                            </select>
                        </div>

                        <div class="form-group">
                            <label>"Notes (optional)"</label>
                            <input type="text"
                                prop:value=move || notes_val.get()
                                on:input=move |e| notes_val.set(event_target_value(&e))
                                on:blur={let s = save.clone(); move |_| s()}
                                placeholder="e.g. per side"
                            />
                        </div>

                        <button class="btn btn-ghost btn-sm" style="color:var(--danger)" on:click=delete_ex>
                            "Delete exercise"
                        </button>
                    </div>
                }
            })}
        </div>
    }
}

// ── Import / Export ───────────────────────────────────────────────────────────

#[component]
pub fn ImportExportView() -> impl IntoView {
    let state = expect_context::<AppState>();
    let import_text = RwSignal::new(String::new());
    let import_error = RwSignal::new(Option::<String>::None);

    let export_plan = move |_| {
        let csv = export_plan_csv(&state.plan.get());
        download_file("workout_plan.csv", &csv);
    };

    let do_import = move |_| {
        let text = import_text.get();
        match import_plan_csv(&text) {
            Ok(plan) => {
                state.plan.set(plan);
                import_text.set(String::new());
                import_error.set(None);
                state.show_toast("Plan imported successfully!");
                state.navigate(View::PlanEditor);
            }
            Err(e) => import_error.set(Some(e)),
        }
    };

    view! {
        <div class="page">
            <div class="page-header">
                <button class="back-btn" on:click=move |_| state.navigate(View::PlanEditor)>
                    "‹ Back"
                </button>
                <h1 class="page-title">"Import / Export"</h1>
            </div>

            <div class="card">
                <div class="card-title">"Export Plan"</div>
                <div class="card-sub" style="margin-bottom:12px">
                    "Download your current workout plan as CSV. You can email it to yourself for backup."
                </div>
                <button class="btn btn-primary" on:click=export_plan>"Download Plan CSV"</button>
            </div>

            <div class="card">
                <div class="card-title">"Import Plan"</div>
                <div class="card-sub" style="margin-bottom:12px">
                    "Paste a CSV in the format: day_id, day_name, exercise_id, exercise_name, target_sets, reps_min, reps_max, category, notes"
                </div>
                <div class="form-group">
                    <label>"Paste CSV content"</label>
                    <textarea
                        rows="8"
                        style="width:100%; resize:vertical; font-family:monospace; font-size:12px"
                        prop:value=move || import_text.get()
                        on:input=move |e| import_text.set(event_target_value(&e))
                        placeholder="day_id,day_name,exercise_id,exercise_name,target_sets,reps_min,reps_max,category,notes\nd1,Lower A,d1-e1,Hip thrust,4,6,10,Main,"
                    />
                </div>
                {move || import_error.get().map(|e| view! {
                    <p style="color:var(--danger); font-size:13px; margin-bottom:8px">{e}</p>
                })}
                <button
                    class="btn btn-primary"
                    on:click=do_import
                    disabled=move || import_text.get().trim().is_empty()
                >
                    "Import Plan"
                </button>
                <p class="text-sm text-muted" style="margin-top:8px">
                    "⚠ This will replace your current plan."
                </p>
            </div>
        </div>
    }
}

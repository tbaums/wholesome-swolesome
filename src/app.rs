use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::models::{Exercise, ExerciseEntry, ExerciseLog, SetLog, WorkoutSession};
use crate::storage;
use crate::sync;

// ── Navigation ────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Debug)]
pub enum View {
    Home,
    Session { day_id: String },
    Exercises,
    History,
    SessionDetail { session_id: String },
    PlanEditor,
    DayEditor { day_id: String },
    Progress { exercise_name: String },
    ImportExport,
    Options,
}

// ── Global state ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct AppState {
    pub plan: RwSignal<crate::models::WorkoutPlan>,
    pub history: RwSignal<Vec<ExerciseEntry>>,
    pub active_session: RwSignal<Option<WorkoutSession>>,
    pub session_drafts: RwSignal<Vec<WorkoutSession>>,
    pub custom_exercises: RwSignal<Vec<Exercise>>,
    pub view: RwSignal<View>,
    pub toast: RwSignal<Option<String>>,
    pub sync_sha: RwSignal<Option<String>>,
    pub last_synced_at: RwSignal<Option<String>>,
    pub suppress_push: RwSignal<bool>,
}

impl AppState {
    pub fn navigate(&self, v: View) {
        self.view.set(v);
    }

    pub fn show_toast(&self, msg: impl Into<String>) {
        let toast = self.toast;
        toast.set(Some(msg.into()));

        let cb = Closure::once(move || toast.set(None));
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref::<js_sys::Function>(),
                2500,
            );
        }
        cb.forget();
    }
}

// ── App root ──────────────────────────────────────────────────────────────────

#[component]
pub fn App() -> impl IntoView {
    let initial_session = storage::load_active_session();
    let initial_view = if initial_session.is_some() {
        View::Session { day_id: String::new() }
    } else {
        View::Home
    };

    let state = AppState {
        plan: RwSignal::new(storage::load_plan()),
        history: RwSignal::new(storage::load_exercise_history()),
        active_session: RwSignal::new(initial_session),
        session_drafts: RwSignal::new(storage::load_session_drafts()),
        custom_exercises: RwSignal::new(storage::load_custom_exercises()),
        view: RwSignal::new(initial_view),
        toast: RwSignal::new(None),
        sync_sha: RwSignal::new(None),
        last_synced_at: RwSignal::new(storage::load_last_push_at()),
        suppress_push: RwSignal::new(false),
    };
    provide_context(state);

    // Becomes true once the boot pull attempt completes (whether or not it pulled).
    // The debounced push Effect checks this to avoid pushing data before pull finishes.
    let boot_done = RwSignal::new(false);
    let debounce_handle: StoredValue<Option<i32>> = StoredValue::new(None);

    // Boot-time pull: updates signals directly. boot_done is false during hydration
    // so the debounce Effect ignores these signal changes.
    spawn_local(async move {
        let cfg = storage::load_sync_config();
        if cfg.is_configured() {
            match sync::fetch_state(&cfg.to_github_config()).await {
                Ok(remote) => {
                    state.sync_sha.set(Some(remote.sha));
                    let Some(remote_ts) = remote.state.updated_at.as_deref() else {
                        boot_done.set(true);
                        return;
                    };
                    let should_hydrate = match storage::load_last_push_at().as_deref() {
                        None => true,
                        Some(local_ts) => remote_ts > local_ts,
                    };
                    if should_hydrate {
                        if let Some(plan) = remote.state.plan {
                            state.plan.set(plan);
                        }
                        if !remote.state.exercise_history.is_empty() {
                            state.history.set(remote.state.exercise_history);
                        }
                        if !remote.state.session_drafts.is_empty() {
                            state.session_drafts.set(remote.state.session_drafts);
                        }
                        if !remote.state.custom_exercises.is_empty() {
                            state.custom_exercises.set(remote.state.custom_exercises);
                        }
                        storage::save_last_push_at(remote_ts);
                        state.last_synced_at.set(Some(remote_ts.to_string()));
                        state.show_toast("Synced from GitHub ↓");
                    }
                }
                Err(sync::SyncError::NotFound) => {}
                Err(e) => leptos::logging::warn!("Boot sync pull failed: {e}"),
            }
        }
        boot_done.set(true);
    });

    Effect::new(move |_| { storage::save_plan(&state.plan.get()); });
    Effect::new(move |_| { storage::save_exercise_history(&state.history.get()); });
    Effect::new(move |_| { storage::save_active_session(&state.active_session.get()); });
    Effect::new(move |_| { storage::save_session_drafts(&state.session_drafts.get()); });
    Effect::new(move |_| { storage::save_custom_exercises(&state.custom_exercises.get()); });

    // Debounced push: 2s after any data signal changes, push to GitHub.
    // Skips first run (initial render from localStorage) and skips until boot_done.
    Effect::new(move |prev: Option<()>| {
        let _ = (
            state.plan.get(),
            state.history.get(),
            state.session_drafts.get(),
            state.custom_exercises.get(),
        );
        if prev.is_none() || !boot_done.get_untracked() || state.suppress_push.get_untracked() {
            return;
        }
        if let Some(handle) = debounce_handle.get_value() {
            if let Some(window) = web_sys::window() {
                window.clear_timeout_with_handle(handle);
            }
        }
        let cfg = storage::load_sync_config();
        if !cfg.is_configured() {
            return;
        }
        let cb = Closure::once(move || {
            let synced = build_synced_state(state);
            let gh_cfg = cfg.to_github_config();
            let sha = state.sync_sha.get_untracked();
            spawn_local(async move {
                match sync::push_state(&gh_cfg, &synced, sha.as_deref()).await {
                    Ok(new_sha) => {
                        state.sync_sha.set(Some(new_sha));
                        let ts = current_datetime();
                        storage::save_last_push_at(&ts);
                        state.last_synced_at.set(Some(ts));
                    }
                    Err(sync::SyncError::Conflict) => {
                        if let Ok(remote) = sync::fetch_state(&gh_cfg).await {
                            let new_sha = remote.sha.clone();
                            state.sync_sha.set(Some(new_sha.clone()));
                            if let Ok(s) = sync::push_state(&gh_cfg, &synced, Some(&new_sha)).await {
                                state.sync_sha.set(Some(s));
                                let ts = current_datetime();
                                storage::save_last_push_at(&ts);
                                state.last_synced_at.set(Some(ts));
                            }
                        }
                    }
                    Err(e) => leptos::logging::warn!("Auto-push failed: {e}"),
                }
            });
        });
        let handle = web_sys::window().and_then(|w| {
            w.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref::<js_sys::Function>(),
                2000,
            ).ok()
        });
        cb.forget();
        debounce_handle.set_value(handle);
    });

    view! {
        <div id="app">
            <CurrentView/>
            <BottomNav/>
            <Toast/>
        </div>
    }
}

pub fn build_synced_state(state: AppState) -> sync::SyncedState {
    sync::SyncedState {
        schema_version: 1,
        updated_at: Some(current_datetime()),
        plan: Some(state.plan.get_untracked()),
        exercise_history: state.history.get_untracked(),
        session_drafts: state.session_drafts.get_untracked(),
        custom_exercises: state.custom_exercises.get_untracked(),
    }
}

// ── Router ────────────────────────────────────────────────────────────────────

#[component]
fn CurrentView() -> impl IntoView {
    let state = expect_context::<AppState>();

    move || match state.view.get() {
        View::Home => view! { <crate::components::home::HomeView/> }.into_any(),
        View::Session { day_id } => {
            view! { <crate::components::session::SessionView _day_id=day_id/> }.into_any()
        }
        View::Exercises => view! { <crate::components::exercises::ExercisesView/> }.into_any(),
        View::History => view! { <crate::components::history::HistoryView/> }.into_any(),
        View::SessionDetail { session_id } => {
            view! { <crate::components::history::SessionDetailView session_id=session_id/> }
                .into_any()
        }
        View::PlanEditor => view! { <crate::components::plan_editor::PlanEditorView/> }.into_any(),
        View::DayEditor { day_id } => {
            view! { <crate::components::plan_editor::DayEditorView day_id=day_id/> }.into_any()
        }
        View::Progress { exercise_name } => {
            view! { <crate::components::progress::ProgressView exercise_name=exercise_name/> }
                .into_any()
        }
        View::ImportExport => {
            view! { <crate::components::plan_editor::ImportExportView/> }.into_any()
        }
        View::Options => view! { <crate::components::options::OptionsView/> }.into_any(),
    }
}

// ── Bottom nav ────────────────────────────────────────────────────────────────

#[component]
fn BottomNav() -> impl IntoView {
    let state = expect_context::<AppState>();
    let view = state.view;

    let is_home = move || matches!(view.get(), View::Home | View::Session { .. });
    let is_exercises = move || matches!(view.get(), View::Exercises);
    let is_history = move || {
        matches!(
            view.get(),
            View::History | View::SessionDetail { .. } | View::Progress { .. } | View::Options
        )
    };
    let is_plan = move || matches!(view.get(), View::PlanEditor | View::DayEditor { .. } | View::ImportExport);

    view! {
        <nav class="bottom-nav">
            // Workout — dumbbell
            <button class="nav-btn" class:active=is_home on:click=move |_| {
                state.navigate(View::Home);
            }>
                <span class="icon">
                    <svg width="24" height="24" attr:viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                        <path d="M6.5 8v8M17.5 8v8M3 10v4M21 10v4M6.5 12h11"/>
                    </svg>
                </span>
                <span>"Workout"</span>
            </button>
            // Exercises — list
            <button class="nav-btn" class:active=is_exercises on:click=move |_| state.navigate(View::Exercises)>
                <span class="icon">
                    <svg width="24" height="24" attr:viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <line x1="8" y1="6" x2="21" y2="6"/>
                        <line x1="8" y1="12" x2="21" y2="12"/>
                        <line x1="8" y1="18" x2="21" y2="18"/>
                        <line x1="3" y1="6" x2="3.01" y2="6"/>
                        <line x1="3" y1="12" x2="3.01" y2="12"/>
                        <line x1="3" y1="18" x2="3.01" y2="18"/>
                    </svg>
                </span>
                <span>"Exercises"</span>
            </button>
            // Plan — clipboard
            <button class="nav-btn" class:active=is_plan on:click=move |_| state.navigate(View::PlanEditor)>
                <span class="icon">
                    <svg width="24" height="24" attr:viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2"/>
                        <rect x="9" y="3" width="6" height="4" rx="1"/>
                        <path d="M9 12h6M9 16h4"/>
                    </svg>
                </span>
                <span>"Plan"</span>
            </button>
            // History — trending up
            <button class="nav-btn" class:active=is_history on:click=move |_| state.navigate(View::History)>
                <span class="icon">
                    <svg width="24" height="24" attr:viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="22 7 13.5 15.5 8.5 10.5 2 17"/>
                        <polyline points="16 7 22 7 22 13"/>
                    </svg>
                </span>
                <span>"History"</span>
            </button>
        </nav>
    }
}

// ── Toast ─────────────────────────────────────────────────────────────────────

#[component]
fn Toast() -> impl IntoView {
    let state = expect_context::<AppState>();
    move || {
        state.toast.get().map(|msg| {
            view! { <div class="toast">{msg}</div> }
        })
    }
}

// ── Session factory ───────────────────────────────────────────────────────────

/// Creates a new WorkoutSession for `day_id`, pre-filling weights/reps from the
/// most recent session for that day (looked up from the flat ExerciseEntry history).
pub fn new_session(
    day_id: &str,
    plan: &crate::models::WorkoutPlan,
    history: &[ExerciseEntry],
) -> Option<WorkoutSession> {
    let day = plan.days.iter().find(|d| d.id == day_id)?;

    // Find the most recent session_id for this day
    let last_session_id = history
        .iter()
        .rev()
        .filter(|e| e.day_id.as_deref() == Some(day_id))
        .filter_map(|e| e.session_id.as_deref())
        .next()
        .map(|s| s.to_string());

    let exercise_logs: Vec<ExerciseLog> = day
        .exercises
        .iter()
        .map(|ex| {
            let (default_weight, default_reps) = last_session_id
                .as_deref()
                .and_then(|sid| {
                    history
                        .iter()
                        .find(|e| {
                            e.session_id.as_deref() == Some(sid) && e.exercise_id == ex.id
                        })
                })
                .and_then(|e| e.sets.iter().filter(|s| s.completed).last())
                .map(|s| (s.weight, s.reps))
                .unwrap_or((0.0, ex.reps_min));

            let sets = (1..=ex.target_sets)
                .map(|n| SetLog {
                    set_number: n,
                    reps: default_reps,
                    weight: default_weight,
                    completed: false,
                    completed_date: None,
                })
                .collect();

            ExerciseLog {
                exercise_id: ex.id.clone(),
                exercise_name: ex.name.clone(),
                target_sets: ex.target_sets,
                reps_min: ex.reps_min,
                reps_max: ex.reps_max,
                sets,
            }
        })
        .collect();

    Some(WorkoutSession {
        id: uuid::Uuid::new_v4().to_string(),
        date: current_date(),
        day_id: day.id.clone(),
        day_name: day.name.clone(),
        exercise_logs,
        is_complete: false,
    })
}

pub fn current_date() -> String {
    let date = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date()
    )
}

pub fn current_datetime() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}

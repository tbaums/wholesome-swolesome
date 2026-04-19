use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::models::{ExerciseLog, SetLog, WorkoutSession};
use crate::storage;

// ── Navigation ────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Debug)]
pub enum View {
    Home,
    Session { day_id: String },
    History,
    SessionDetail { session_id: String },
    PlanEditor,
    DayEditor { day_id: String },
    Progress { exercise_name: String },
    ImportExport,
}

// ── Global state ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct AppState {
    pub plan: RwSignal<crate::models::WorkoutPlan>,
    pub history: RwSignal<Vec<WorkoutSession>>,
    pub active_session: RwSignal<Option<WorkoutSession>>,
    pub view: RwSignal<View>,
    pub toast: RwSignal<Option<String>>,
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
        history: RwSignal::new(storage::load_history()),
        active_session: RwSignal::new(initial_session),
        view: RwSignal::new(initial_view),
        toast: RwSignal::new(None),
    };
    provide_context(state);

    // Auto-save plan whenever it changes
    Effect::new(move |_| {
        storage::save_plan(&state.plan.get());
    });

    // Auto-save history whenever it changes
    Effect::new(move |_| {
        storage::save_history(&state.history.get());
    });

    // Auto-save active session on every change; clear when finished
    Effect::new(move |_| {
        storage::save_active_session(&state.active_session.get());
    });

    view! {
        <div id="app">
            <CurrentView/>
            <BottomNav/>
            <Toast/>
        </div>
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
    }
}

// ── Bottom nav ────────────────────────────────────────────────────────────────

#[component]
fn BottomNav() -> impl IntoView {
    let state = expect_context::<AppState>();
    let view = state.view;

    let is_home = move || matches!(view.get(), View::Home | View::Session { .. });
    let is_history = move || {
        matches!(view.get(), View::History | View::SessionDetail { .. } | View::Progress { .. })
    };
    let is_plan = move || matches!(view.get(), View::PlanEditor | View::DayEditor { .. } | View::ImportExport);

    view! {
        <nav class="bottom-nav">
            // Workout — dumbbell
            <button class="nav-btn" class:active=is_home on:click=move |_| {
                if state.active_session.get_untracked().is_some() {
                    state.navigate(View::Session { day_id: String::new() });
                } else {
                    state.navigate(View::Home);
                }
            }>
                <span class="icon">
                    <svg width="24" height="24" attr:viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                        <path d="M6.5 8v8M17.5 8v8M3 10v4M21 10v4M6.5 12h11"/>
                    </svg>
                </span>
                <span>"Workout"</span>
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
/// most recent session for that day.
pub fn new_session(
    day_id: &str,
    plan: &crate::models::WorkoutPlan,
    history: &[WorkoutSession],
) -> Option<WorkoutSession> {
    let day = plan.days.iter().find(|d| d.id == day_id)?;

    let last = history
        .iter()
        .rev()
        .find(|s| s.day_id == day_id);

    let exercise_logs: Vec<ExerciseLog> = day
        .exercises
        .iter()
        .map(|ex| {
            let last_log = last.and_then(|s| {
                s.exercise_logs
                    .iter()
                    .find(|l| l.exercise_id == ex.id)
            });

            // Use the last completed set's values as defaults
            let (default_weight, default_reps) = last_log
                .and_then(|l| l.sets.iter().filter(|s| s.completed).last())
                .map(|s| (s.weight_lbs, s.reps))
                .unwrap_or((0.0, ex.reps_min));

            let sets = (1..=ex.target_sets)
                .map(|n| SetLog {
                    set_number: n,
                    reps: default_reps,
                    weight_lbs: default_weight,
                    completed: false,
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

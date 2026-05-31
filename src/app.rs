use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::library;
use crate::models::{
    Exercise, ExerciseEntry, ExerciseLog, LibraryExercise, ScheduledWorkout, SetLog,
    UserGoals, WorkoutSession,
};
use crate::storage;
use crate::sync;

// ── Navigation ────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Debug)]
pub enum View {
    Home,
    Session { workout_id: String },
    Exercises,
    Library,
    LibraryDetail { exercise_id: String, from: Option<Box<View>> },
    History,
    SessionDetail { session_id: String },
    Progress { exercise_name: String },
    Options,
    CoachPacket,
}

// ── Global state ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct AppState {
    pub goals: RwSignal<UserGoals>,
    pub scheduled_workouts: RwSignal<Vec<ScheduledWorkout>>,
    pub history: RwSignal<Vec<ExerciseEntry>>,
    pub active_session: RwSignal<Option<WorkoutSession>>,
    pub session_drafts: RwSignal<Vec<WorkoutSession>>,
    pub custom_exercises: RwSignal<Vec<Exercise>>,
    pub library: RwSignal<Vec<LibraryExercise>>,
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
    let initial_view = if let Some(ref s) = initial_session {
        View::Session { workout_id: s.day_id.clone() }
    } else {
        View::Home
    };

    let state = AppState {
        goals: RwSignal::new(storage::load_goals()),
        scheduled_workouts: RwSignal::new(storage::load_scheduled_workouts()),
        history: RwSignal::new(storage::load_exercise_history()),
        active_session: RwSignal::new(initial_session),
        session_drafts: RwSignal::new(storage::load_session_drafts()),
        custom_exercises: RwSignal::new(storage::load_custom_exercises()),
        library: RwSignal::new(Vec::new()),
        view: RwSignal::new(initial_view),
        toast: RwSignal::new(None),
        sync_sha: RwSignal::new(None),
        last_synced_at: RwSignal::new(storage::load_last_push_at()),
        suppress_push: RwSignal::new(false),
    };
    provide_context(state);

    // Fetch exercise library asynchronously — non-blocking.
    spawn_local(async move {
        match library::fetch_library().await {
            Ok(lib) => state.library.set(lib),
            Err(e) => leptos::logging::warn!("Library load failed: {e}"),
        }
    });

    let boot_done = RwSignal::new(false);
    let debounce_handle: StoredValue<Option<i32>> = StoredValue::new(None);

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
                        state.goals.set(remote.state.goals);
                        state.scheduled_workouts.set(remote.state.scheduled_workouts);
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

    Effect::new(move |_| { storage::save_goals(&state.goals.get()); });
    Effect::new(move |_| { storage::save_scheduled_workouts(&state.scheduled_workouts.get()); });
    Effect::new(move |_| { storage::save_exercise_history(&state.history.get()); });
    Effect::new(move |_| { storage::save_active_session(&state.active_session.get()); });
    Effect::new(move |_| { storage::save_session_drafts(&state.session_drafts.get()); });
    Effect::new(move |_| { storage::save_custom_exercises(&state.custom_exercises.get()); });

    Effect::new(move |prev: Option<()>| {
        let _ = (
            state.goals.get(),
            state.scheduled_workouts.get(),
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
        schema_version: 2,
        updated_at: Some(current_datetime()),
        goals: state.goals.get_untracked(),
        scheduled_workouts: state.scheduled_workouts.get_untracked(),
        exercise_history: state.history.get_untracked(),
        session_drafts: state.session_drafts.get_untracked(),
        custom_exercises: state.custom_exercises.get_untracked(),
        plan: None,
    }
}

// ── Router ────────────────────────────────────────────────────────────────────

#[component]
fn CurrentView() -> impl IntoView {
    let state = expect_context::<AppState>();

    move || match state.view.get() {
        View::Home => view! { <crate::components::home::HomeView/> }.into_any(),
        View::Session { workout_id } => {
            view! { <crate::components::session::SessionView _workout_id=workout_id/> }.into_any()
        }
        View::Exercises => view! { <crate::components::exercises::ExercisesView/> }.into_any(),
        View::Library => view! { <crate::components::library_view::LibraryView/> }.into_any(),
        View::LibraryDetail { exercise_id, from } => {
            view! { <crate::components::library_view::LibraryDetailView exercise_id=exercise_id from=from/> }
                .into_any()
        }
        View::History => view! { <crate::components::history::HistoryView/> }.into_any(),
        View::SessionDetail { session_id } => {
            view! { <crate::components::history::SessionDetailView session_id=session_id/> }
                .into_any()
        }
        View::Progress { exercise_name } => {
            view! { <crate::components::progress::ProgressView exercise_name=exercise_name/> }
                .into_any()
        }
        View::Options => view! { <crate::components::options::OptionsView/> }.into_any(),
        View::CoachPacket => view! { <crate::components::options::CoachPacketView/> }.into_any(),
    }
}

// ── Bottom nav ────────────────────────────────────────────────────────────────

#[component]
fn BottomNav() -> impl IntoView {
    let state = expect_context::<AppState>();
    let view = state.view;

    let is_home = move || matches!(view.get(), View::Home | View::Session { .. });
    let is_library = move || matches!(view.get(), View::Library | View::LibraryDetail { .. });
    let is_exercises = move || matches!(view.get(), View::Exercises);
    let is_history = move || {
        matches!(
            view.get(),
            View::History
                | View::SessionDetail { .. }
                | View::Progress { .. }
                | View::Options
                | View::CoachPacket
        )
    };

    view! {
        <nav class="bottom-nav">
            // Workout — dumbbell
            <button class="nav-btn" class:active=is_home on:click=move |_| {
                state.navigate(View::Home);
            }>
                <span class="icon">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                        <path d="M6.5 8v8M17.5 8v8M3 10v4M21 10v4M6.5 12h11"/>
                    </svg>
                </span>
                <span>"Workout"</span>
            </button>
            // Library — book
            <button class="nav-btn" class:active=is_library on:click=move |_| state.navigate(View::Library)>
                <span class="icon">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M4 4.5A2.5 2.5 0 016.5 2H20v20H6.5A2.5 2.5 0 014 19.5v-15z"/>
                        <path d="M4 19.5A2.5 2.5 0 016.5 17H20"/>
                    </svg>
                </span>
                <span>"Library"</span>
            </button>
            // Exercises — list (freeform logging)
            <button class="nav-btn" class:active=is_exercises on:click=move |_| state.navigate(View::Exercises)>
                <span class="icon">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
            // History — trending up
            <button class="nav-btn" class:active=is_history on:click=move |_| state.navigate(View::History)>
                <span class="icon">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
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

/// Builds a WorkoutSession from a ScheduledWorkout, pre-filling weights/reps
/// from the most recent matching ExerciseEntry in history (matched by
/// library_id when available, else by exercise name).
pub fn new_session_from_scheduled(
    workout: &ScheduledWorkout,
    history: &[ExerciseEntry],
    library: &[crate::models::LibraryExercise],
) -> WorkoutSession {
    let exercise_logs: Vec<ExerciseLog> = workout
        .exercises
        .iter()
        .map(|ex| {
            let is_duration = ex.target_duration_seconds.is_some();
            let is_cardio = crate::library::is_cardio_exercise(
                ex.library_id.as_deref().unwrap_or(""),
                &ex.name,
                library,
            );
            let prev_sets = if is_duration {
                Vec::new()
            } else {
                last_completed_sets(history, ex)
            };
            // Step-up: if last session hit the top of the rep range on every
            // completed set, bump suggested weight by one barbell increment.
            // Skip for cardio — `weight` there is RPE 1-10, not load.
            let bump = if !is_cardio && should_step_up(&prev_sets, ex.reps_max) {
                5.0
            } else {
                0.0
            };

            let sets = (1..=ex.target_sets)
                .map(|n| {
                    // reps come from the prescription (target lower bound).
                    // Coach-prescribed reps are the point; the user can adjust
                    // mid-session, but the starting value is what was planned.
                    let reps = ex.reps_min;
                    // weight still pre-fills from history (coach doesn't
                    // prescribe weight — it depends on progressive overload).
                    let prev_weight = prev_sets
                        .iter()
                        .find(|s| s.set_number == n)
                        .or_else(|| prev_sets.last())
                        .map(|s| s.weight)
                        .unwrap_or(0.0);
                    let weight = if prev_weight > 0.0 { prev_weight + bump } else { 0.0 };
                    SetLog {
                        set_number: n,
                        reps,
                        weight,
                        completed: false,
                        completed_date: None,
                        duration_seconds: ex.target_duration_seconds,
                    }
                })
                .collect();

            ExerciseLog {
                exercise_id: ex.library_id.clone().unwrap_or_else(|| ex.name.clone()),
                exercise_name: ex.name.clone(),
                target_sets: ex.target_sets,
                reps_min: ex.reps_min,
                reps_max: ex.reps_max,
                sets,
                target_duration_seconds: ex.target_duration_seconds,
            }
        })
        .collect();

    WorkoutSession {
        id: uuid::Uuid::new_v4().to_string(),
        date: current_date(),
        day_id: workout.id.clone(),
        day_name: workout.name.clone(),
        exercise_logs,
        is_complete: false,
    }
}

fn last_completed_sets(
    history: &[ExerciseEntry],
    ex: &crate::models::ScheduledExercise,
) -> Vec<SetLog> {
    let name_lc = ex.name.to_lowercase();
    let entry = history.iter().rev().find(|e| {
        match ex.library_id.as_deref() {
            Some(id) if !id.is_empty() => e.exercise_id == id,
            _ => e.exercise_name.to_lowercase() == name_lc,
        }
    });
    match entry {
        Some(e) => e.sets.iter().filter(|s| s.completed).cloned().collect(),
        None => Vec::new(),
    }
}

/// Progressive-overload trigger: every completed set in the last session
/// hit or exceeded the top of the prescribed rep range, and there was at
/// least one such set. Empty input or any miss → no step-up.
pub fn should_step_up(prev_sets: &[SetLog], reps_max: u32) -> bool {
    !prev_sets.is_empty() && prev_sets.iter().all(|s| s.reps >= reps_max)
}

pub fn current_date() -> String {
    let date = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date(),
    )
}

pub fn current_datetime() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ScheduledExercise, WorkoutSource};
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    fn sched_workout(exs: Vec<ScheduledExercise>) -> ScheduledWorkout {
        ScheduledWorkout {
            id: "w1".into(),
            date: "2026-05-28".into(),
            name: "Test Day".into(),
            rationale: String::new(),
            source: WorkoutSource::Manual,
            exercises: exs,
            created_at: String::new(),
        }
    }

    fn sched_ex(library_id: Option<&str>, name: &str, target_sets: u32) -> ScheduledExercise {
        ScheduledExercise {
            library_id: library_id.map(String::from),
            name: name.into(),
            target_sets,
            reps_min: 8,
            reps_max: 12,
            rest_seconds: 90,
            notes: None,
            target_duration_seconds: None,
        }
    }

    fn set(n: u32, weight: f32, reps: u32, completed: bool) -> SetLog {
        SetLog {
            set_number: n,
            reps,
            weight,
            completed,
            completed_date: None,
            duration_seconds: None,
        }
    }

    fn entry(
        exercise_id: &str,
        exercise_name: &str,
        date: &str,
        sets: Vec<SetLog>,
    ) -> ExerciseEntry {
        ExerciseEntry {
            id: format!("e-{date}-{exercise_id}"),
            date: date.into(),
            exercise_name: exercise_name.into(),
            exercise_id: exercise_id.into(),
            session_id: None,
            day_id: None,
            day_name: None,
            target_sets: sets.len() as u32,
            reps_min: 8,
            reps_max: 12,
            sets,
            finalized: true,
            created_at: String::new(),
            target_duration_seconds: None,
        }
    }

    #[wasm_bindgen_test]
    fn prefill_weight_per_set_reps_from_prescription() {
        // Last session: progressive ramp under reps_max (12), so no step-up.
        let history = vec![entry(
            "Barbell_Bench_Press_-_Medium_Grip",
            "Bench Press",
            "2026-05-20",
            vec![
                set(1, 135.0, 10, true),
                set(2, 145.0, 8, true),
                set(3, 155.0, 6, true),
            ],
        )];
        let workout = sched_workout(vec![sched_ex(
            Some("Barbell_Bench_Press_-_Medium_Grip"),
            "Bench Press",
            3,
        )]);

        let session = new_session_from_scheduled(&workout, &history, &[]);
        let sets = &session.exercise_logs[0].sets;

        // Weights still pre-fill from history (per-set match).
        assert_eq!(sets[0].weight, 135.0);
        assert_eq!(sets[1].weight, 145.0);
        assert_eq!(sets[2].weight, 155.0);
        // Reps now come from the prescription (reps_min=8 for every set),
        // not from what the user did last time.
        assert_eq!(sets[0].reps, 8);
        assert_eq!(sets[1].reps, 8);
        assert_eq!(sets[2].reps, 8);
    }

    #[wasm_bindgen_test]
    fn prefill_weight_falls_back_to_last_prior_set_when_target_has_more_sets() {
        let history = vec![entry(
            "Barbell_Squat",
            "Squat",
            "2026-05-20",
            vec![set(1, 185.0, 5, true), set(2, 205.0, 5, true)],
        )];
        // Asking for 4 sets but only 2 were logged previously.
        let workout = sched_workout(vec![sched_ex(Some("Barbell_Squat"), "Squat", 4)]);

        let session = new_session_from_scheduled(&workout, &history, &[]);
        let sets = &session.exercise_logs[0].sets;

        // Weights: per-set match where available, else last prior set.
        assert_eq!(sets[0].weight, 185.0);
        assert_eq!(sets[1].weight, 205.0);
        assert_eq!(sets[2].weight, 205.0);
        assert_eq!(sets[3].weight, 205.0);
        // Reps always from prescription.
        for s in sets {
            assert_eq!(s.reps, 8);
        }
    }

    #[wasm_bindgen_test]
    fn prefill_defaults_to_zero_weight_and_reps_min_when_no_history() {
        let workout = sched_workout(vec![sched_ex(Some("Deadlift"), "Deadlift", 3)]);

        let session = new_session_from_scheduled(&workout, &[], &[]);
        let sets = &session.exercise_logs[0].sets;

        for s in sets {
            assert_eq!(s.weight, 0.0);
            assert_eq!(s.reps, 8); // reps_min from sched_ex helper
            assert!(!s.completed);
        }
    }

    #[wasm_bindgen_test]
    fn prefill_weight_ignores_uncompleted_prior_sets() {
        let history = vec![entry(
            "Row",
            "Row",
            "2026-05-20",
            vec![
                set(1, 95.0, 10, true),
                set(2, 105.0, 8, true),
                set(3, 115.0, 6, false), // missed top set — excluded
            ],
        )];
        let workout = sched_workout(vec![sched_ex(None, "Row", 3)]);

        let session = new_session_from_scheduled(&workout, &history, &[]);
        let sets = &session.exercise_logs[0].sets;

        assert_eq!(sets[0].weight, 95.0);
        assert_eq!(sets[1].weight, 105.0);
        // Set 3 should fall back to the last *completed* prior set (set 2: 105),
        // not the missed 115.
        assert_eq!(sets[2].weight, 105.0);
    }

    #[wasm_bindgen_test]
    fn prefill_matches_by_library_id_when_present() {
        let history = vec![entry(
            "Barbell_Bench_Press_-_Medium_Grip",
            "Whatever The User Renamed It",
            "2026-05-20",
            vec![set(1, 200.0, 5, true)],
        )];
        let workout = sched_workout(vec![sched_ex(
            Some("Barbell_Bench_Press_-_Medium_Grip"),
            "Bench Press",
            1,
        )]);

        let session = new_session_from_scheduled(&workout, &history, &[]);
        assert_eq!(session.exercise_logs[0].sets[0].weight, 200.0);
    }

    // ── Step-up (progressive overload nudge) ──────────────────────────────────

    #[wasm_bindgen_test]
    fn step_up_bumps_weight_when_every_set_hit_reps_max() {
        // Last session: 3 sets all at reps_max (12). Trigger.
        let history = vec![entry(
            "Press",
            "Press",
            "2026-05-20",
            vec![
                set(1, 100.0, 12, true),
                set(2, 100.0, 12, true),
                set(3, 100.0, 12, true),
            ],
        )];
        let workout = sched_workout(vec![sched_ex(None, "Press", 3)]);
        let session = new_session_from_scheduled(&workout, &history, &[]);
        for s in &session.exercise_logs[0].sets {
            assert_eq!(s.weight, 105.0, "every set should bump by 5 lb");
        }
    }

    #[wasm_bindgen_test]
    fn step_up_skips_when_any_set_short_of_reps_max() {
        let history = vec![entry(
            "Press",
            "Press",
            "2026-05-20",
            vec![
                set(1, 100.0, 12, true),
                set(2, 100.0, 12, true),
                set(3, 100.0, 10, true), // shy of reps_max=12 → no step-up
            ],
        )];
        let workout = sched_workout(vec![sched_ex(None, "Press", 3)]);
        let session = new_session_from_scheduled(&workout, &history, &[]);
        for s in &session.exercise_logs[0].sets {
            assert_eq!(s.weight, 100.0, "no bump when last session missed top");
        }
    }

    #[wasm_bindgen_test]
    fn step_up_skips_when_no_prior_weight() {
        // First-ever session for this exercise: no bump even if "reps_max" condition would match.
        let workout = sched_workout(vec![sched_ex(None, "Press", 3)]);
        let session = new_session_from_scheduled(&workout, &[], &[]);
        for s in &session.exercise_logs[0].sets {
            assert_eq!(s.weight, 0.0);
        }
    }
}

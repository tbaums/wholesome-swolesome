use crate::models::{Exercise, ExerciseEntry, WorkoutPlan, WorkoutSession};

const PLAN_KEY: &str = "ws_plan";
const EXERCISE_HISTORY_KEY: &str = "ws_ex_history";
const SESSION_KEY: &str = "ws_active_session";
const DRAFTS_KEY: &str = "ws_session_drafts";
const CUSTOM_EXERCISES_KEY: &str = "ws_custom_exercises";

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn load_plan() -> WorkoutPlan {
    local_storage()
        .and_then(|s| s.get_item(PLAN_KEY).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub fn save_plan(plan: &WorkoutPlan) {
    if let (Some(storage), Ok(json)) = (local_storage(), serde_json::to_string(plan)) {
        let _ = storage.set_item(PLAN_KEY, &json);
    }
}

pub fn load_exercise_history() -> Vec<ExerciseEntry> {
    local_storage()
        .and_then(|s| s.get_item(EXERCISE_HISTORY_KEY).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub fn save_exercise_history(history: &[ExerciseEntry]) {
    if let (Some(storage), Ok(json)) = (local_storage(), serde_json::to_string(history)) {
        let _ = storage.set_item(EXERCISE_HISTORY_KEY, &json);
    }
}

pub fn load_active_session() -> Option<WorkoutSession> {
    local_storage()
        .and_then(|s| s.get_item(SESSION_KEY).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub fn load_session_drafts() -> Vec<WorkoutSession> {
    local_storage()
        .and_then(|s| s.get_item(DRAFTS_KEY).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub fn save_session_drafts(drafts: &[WorkoutSession]) {
    if let (Some(storage), Ok(json)) = (local_storage(), serde_json::to_string(drafts)) {
        let _ = storage.set_item(DRAFTS_KEY, &json);
    }
}

pub fn load_custom_exercises() -> Vec<Exercise> {
    local_storage()
        .and_then(|s| s.get_item(CUSTOM_EXERCISES_KEY).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub fn save_custom_exercises(exercises: &[Exercise]) {
    if let (Some(storage), Ok(json)) = (local_storage(), serde_json::to_string(exercises)) {
        let _ = storage.set_item(CUSTOM_EXERCISES_KEY, &json);
    }
}

pub fn save_active_session(session: &Option<WorkoutSession>) {
    if let Some(storage) = local_storage() {
        match session {
            Some(s) => {
                if let Ok(json) = serde_json::to_string(s) {
                    let _ = storage.set_item(SESSION_KEY, &json);
                }
            }
            None => { let _ = storage.remove_item(SESSION_KEY); }
        }
    }
}

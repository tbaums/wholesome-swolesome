use crate::models::{WorkoutPlan, WorkoutSession};

const PLAN_KEY: &str = "ws_plan";
const HISTORY_KEY: &str = "ws_history";
const SESSION_KEY: &str = "ws_active_session";

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

pub fn load_history() -> Vec<WorkoutSession> {
    local_storage()
        .and_then(|s| s.get_item(HISTORY_KEY).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub fn save_history(history: &[WorkoutSession]) {
    if let (Some(storage), Ok(json)) = (local_storage(), serde_json::to_string(history)) {
        let _ = storage.set_item(HISTORY_KEY, &json);
    }
}

pub fn load_active_session() -> Option<WorkoutSession> {
    local_storage()
        .and_then(|s| s.get_item(SESSION_KEY).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
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

//! GitHub Contents API sync backend.
//!
//! Single `state.json` file in a private repo, written via PUT with the
//! previous blob sha for optimistic concurrency. Last-write-wins on top of
//! that: on 409/422 the caller should re-fetch and retry.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

use crate::models::{
    Exercise, ExerciseEntry, ScheduledWorkout, UserGoals, WorkoutPlan, WorkoutSession,
};

const API_BASE: &str = "https://api.github.com";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncConfig {
    pub token: String,
    pub repo: String,
    pub branch: String,
    pub path: String,
}

impl SyncConfig {
    pub fn is_configured(&self) -> bool {
        !self.token.is_empty() && !self.repo.is_empty()
    }

    pub fn to_github_config(&self) -> GitHubConfig {
        GitHubConfig {
            token: self.token.clone(),
            repo: self.repo.clone(),
            branch: if self.branch.is_empty() { "main".into() } else { self.branch.clone() },
            path: if self.path.is_empty() { "state.json".into() } else { self.path.clone() },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncedState {
    pub schema_version: u32,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub goals: UserGoals,
    #[serde(default)]
    pub scheduled_workouts: Vec<ScheduledWorkout>,
    #[serde(default)]
    pub exercise_history: Vec<ExerciseEntry>,
    #[serde(default)]
    pub session_drafts: Vec<WorkoutSession>,
    #[serde(default)]
    pub custom_exercises: Vec<Exercise>,
    /// Legacy: preserved on read so old state.json files don't lose data,
    /// but the app no longer uses it. Kept serializing for safety.
    #[serde(default)]
    pub plan: Option<WorkoutPlan>,
}

#[derive(Debug, Clone)]
pub struct GitHubConfig {
    pub token: String,
    pub repo: String,
    pub path: String,
    pub branch: String,
}

#[derive(Debug)]
pub struct RemoteState {
    pub state: SyncedState,
    pub sha: String,
}

#[derive(Debug)]
pub enum SyncError {
    Network(String),
    Http { status: u16, body: String },
    NotFound,
    Conflict,
    Decode(String),
    Serde(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Network(s) => write!(f, "network error: {s}"),
            SyncError::Http { status, body } => write!(f, "HTTP {status}: {body}"),
            SyncError::NotFound => write!(f, "remote file not found"),
            SyncError::Conflict => write!(f, "conflict (sha mismatch)"),
            SyncError::Decode(s) => write!(f, "decode error: {s}"),
            SyncError::Serde(s) => write!(f, "serde error: {s}"),
        }
    }
}

impl std::error::Error for SyncError {}

fn contents_url(cfg: &GitHubConfig) -> String {
    format!("{API_BASE}/repos/{}/contents/{}", cfg.repo, cfg.path)
}

fn auth_header(cfg: &GitHubConfig) -> String {
    format!("Bearer {}", cfg.token)
}

#[derive(Deserialize)]
struct ContentsResp {
    content: String,
    encoding: String,
    sha: String,
}

#[derive(Deserialize)]
struct UpdateResp {
    content: UpdateContent,
}

#[derive(Deserialize)]
struct UpdateContent {
    sha: String,
}

pub async fn fetch_state(cfg: &GitHubConfig) -> Result<RemoteState, SyncError> {
    let url = format!("{}?ref={}", contents_url(cfg), cfg.branch);
    let resp = Request::get(&url)
        .header("Authorization", &auth_header(cfg))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| SyncError::Network(e.to_string()))?;

    let status = resp.status();
    if status == 404 {
        return Err(SyncError::NotFound);
    }
    if !(200..300).contains(&status) {
        let body = resp.text().await.unwrap_or_default();
        return Err(SyncError::Http { status, body });
    }

    let parsed: ContentsResp = resp
        .json()
        .await
        .map_err(|e| SyncError::Decode(e.to_string()))?;

    if parsed.encoding != "base64" {
        return Err(SyncError::Decode(format!(
            "unexpected encoding: {}",
            parsed.encoding
        )));
    }

    // GitHub wraps base64 at 60 chars with newlines; strip whitespace before decoding.
    let cleaned: String = parsed.content.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = BASE64
        .decode(&cleaned)
        .map_err(|e| SyncError::Decode(e.to_string()))?;
    let text = String::from_utf8(bytes).map_err(|e| SyncError::Decode(e.to_string()))?;
    let state: SyncedState =
        serde_json::from_str(&text).map_err(|e| SyncError::Serde(e.to_string()))?;

    Ok(RemoteState {
        state,
        sha: parsed.sha,
    })
}

pub async fn push_state(
    cfg: &GitHubConfig,
    state: &SyncedState,
    prev_sha: Option<&str>,
) -> Result<String, SyncError> {
    let json =
        serde_json::to_string_pretty(state).map_err(|e| SyncError::Serde(e.to_string()))?;
    let content = BASE64.encode(json.as_bytes());

    let mut body = serde_json::json!({
        "message": format!(
            "Update state.json ({})",
            state.updated_at.clone().unwrap_or_else(|| "unknown".into())
        ),
        "content": content,
        "branch": cfg.branch,
    });
    if let Some(sha) = prev_sha {
        body["sha"] = serde_json::Value::String(sha.to_string());
    }

    let resp = Request::put(&contents_url(cfg))
        .header("Authorization", &auth_header(cfg))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| SyncError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| SyncError::Network(e.to_string()))?;

    let status = resp.status();
    if status == 409 || status == 422 {
        return Err(SyncError::Conflict);
    }
    if !(200..300).contains(&status) {
        let body = resp.text().await.unwrap_or_default();
        return Err(SyncError::Http { status, body });
    }

    let parsed: UpdateResp = resp
        .json()
        .await
        .map_err(|e| SyncError::Decode(e.to_string()))?;
    Ok(parsed.content.sha)
}

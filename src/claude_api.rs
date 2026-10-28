//! Optional direct-to-Anthropic path for the three Claude round trips.
//!
//! Everything here is opt-in. Without a stored key the app behaves exactly as
//! it always has: copy a packet, paste it into your own Claude conversation,
//! paste the JSON back. With a key, the same packet and the same prompts are
//! sent straight to `/v1/messages` and handed to the *same* parsers
//! (`coach::parse_workout_response`, `coach::parse_cardio_actuals`), so both
//! paths produce identical imports for identical responses. The copy/paste UI
//! stays on screen either way — it's the fallback when a call fails.
//!
//! The key lives in its own localStorage entry (see `storage::save_claude_config`),
//! deliberately *not* inside `SyncConfig`, so it can never ride along into
//! `SyncedState` → `state.json` → the user's GitHub repo.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// The model every call uses. One constant so the three call sites can't drift.
///
/// The id is the one issue #59 specified. Nothing here validates it: every test
/// mocks `api.anthropic.com` and CI has no key, so the operator smoke test in
/// AC6 is the gate on it being right.
pub const CLAUDE_MODEL: &str = "claude-opus-5-5";

/// Reasoning effort, sent explicitly rather than relying on the default.
/// `medium` is that default and fits plan generation and screenshot reads alike.
pub const CLAUDE_EFFORT: &str = "medium";

const MAX_TOKENS: u32 = 8192;

/// Largest screenshot we'll send. Anthropic's own limit is higher, but a phone
/// screenshot is well under this and rejecting locally saves a doomed upload.
pub const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

/// The user's own Anthropic key. Its own localStorage entry — never nested in
/// [`crate::sync::SyncConfig`] and never a field of `SyncedState`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ClaudeConfig {
    #[serde(default)]
    pub api_key: String,
}

/// A screenshot to send alongside the prompt, already base64-encoded.
#[derive(Debug, Clone)]
pub struct ImagePart {
    pub media_type: String,
    pub data_b64: String,
}

impl ImagePart {
    /// Encode raw file bytes. Returns `None` for an empty or oversized file —
    /// the caller surfaces that as a toast without making a request.
    pub fn from_bytes(media_type: &str, bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
            return None;
        }
        if !media_type.starts_with("image/") {
            return None;
        }
        Some(ImagePart {
            media_type: media_type.to_string(),
            data_b64: BASE64.encode(bytes),
        })
    }
}

#[derive(Debug)]
pub enum ClaudeError {
    /// No key saved, or the saved key is blank.
    NoKey,
    Network(String),
    Http { status: u16, message: String },
    /// `stop_reason: "refusal"` — Claude declined. Import nothing.
    Refusal,
    /// `stop_reason: "max_tokens"` — the response is truncated, so any JSON in
    /// it is likely half-written. Import nothing.
    MaxTokens,
    /// 200 OK but no text block came back.
    EmptyResponse,
    Decode(String),
}

impl std::fmt::Display for ClaudeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClaudeError::NoKey => write!(f, "No Claude API key saved — use the copy/paste flow, or add a key in Options."),
            ClaudeError::Network(e) => write!(f, "Network error: {e}. Copy/paste still works."),
            ClaudeError::Http { status, message } => match status {
                401 => write!(f, "Claude rejected the API key (401). Check it in Options."),
                429 => write!(f, "Claude is rate-limiting this key (429). Try again shortly."),
                _ => {
                    if message.is_empty() {
                        write!(f, "Claude returned HTTP {status}.")
                    } else {
                        write!(f, "Claude returned HTTP {status}: {message}")
                    }
                }
            },
            ClaudeError::Refusal => write!(f, "Claude declined to answer — nothing was imported."),
            ClaudeError::MaxTokens => write!(f, "Claude's response was cut off (max tokens) — nothing was imported."),
            ClaudeError::EmptyResponse => write!(f, "Claude returned no text — nothing was imported."),
            ClaudeError::Decode(e) => write!(f, "Couldn't read Claude's response: {e}"),
        }
    }
}

impl std::error::Error for ClaudeError {}

#[derive(Deserialize)]
struct MessagesResponse {
    #[serde(default)]
    content: Vec<ContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Deserialize)]
struct ApiErrorEnvelope {
    error: ApiErrorBody,
}

#[derive(Deserialize)]
struct ApiErrorBody {
    #[serde(default)]
    message: String,
}

fn request_body(prompt: &str, image: Option<&ImagePart>) -> serde_json::Value {
    // Image first, then the text: Anthropic's guidance is that an image placed
    // before the question it's about reads better.
    let mut content = Vec::new();
    if let Some(img) = image {
        content.push(serde_json::json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": img.media_type,
                "data": img.data_b64,
            }
        }));
    }
    content.push(serde_json::json!({ "type": "text", "text": prompt }));

    // No `thinking: disabled` — thinking is always on for this model — and no
    // assistant prefill, which isn't allowed alongside it.
    serde_json::json!({
        "model": CLAUDE_MODEL,
        "max_tokens": MAX_TOKENS,
        "output_config": { "effort": CLAUDE_EFFORT },
        "messages": [{ "role": "user", "content": content }],
    })
}

/// POST one user turn and return the concatenated text blocks.
///
/// `stop_reason` is checked *before* the text is handed back: a refusal or a
/// truncated response yields an error rather than a partial import.
pub async fn send_message(
    api_key: &str,
    prompt: &str,
    image: Option<&ImagePart>,
) -> Result<String, ClaudeError> {
    if api_key.trim().is_empty() {
        return Err(ClaudeError::NoKey);
    }

    let resp = Request::post(API_URL)
        .header("x-api-key", api_key.trim())
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        // Required for calls made from a browser rather than a server.
        .header("anthropic-dangerous-direct-browser-access", "true")
        .body(request_body(prompt, image).to_string())
        .map_err(|e| ClaudeError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ClaudeError::Network(e.to_string()))?;

    let status = resp.status();
    if !(200..300).contains(&status) {
        let body = resp.text().await.unwrap_or_default();
        let message = serde_json::from_str::<ApiErrorEnvelope>(&body)
            .map(|e| e.error.message)
            .unwrap_or_default();
        return Err(ClaudeError::Http { status, message });
    }

    let text = resp
        .text()
        .await
        .map_err(|e| ClaudeError::Decode(e.to_string()))?;
    parse_messages_response(&text)
}

/// Split out from [`send_message`] so the stop-reason and content-block rules
/// are unit-testable without a network.
pub fn parse_messages_response(body: &str) -> Result<String, ClaudeError> {
    let parsed: MessagesResponse =
        serde_json::from_str(body).map_err(|e| ClaudeError::Decode(e.to_string()))?;

    match parsed.stop_reason.as_deref() {
        Some("refusal") => return Err(ClaudeError::Refusal),
        Some("max_tokens") => return Err(ClaudeError::MaxTokens),
        _ => {}
    }

    // Thinking blocks come back alongside the answer for this model; only the
    // text blocks carry the JSON we parse.
    let text: String = parsed
        .content
        .iter()
        .filter(|b| b.kind == "text")
        .filter_map(|b| b.text.as_deref())
        .collect::<Vec<_>>()
        .join("");

    if text.trim().is_empty() {
        return Err(ClaudeError::EmptyResponse);
    }
    Ok(text)
}

/// Read a picked file into an [`ImagePart`]. `None` means "don't send this" —
/// not an image, empty, or over [`MAX_IMAGE_BYTES`] — and the caller surfaces
/// that as a toast without making a request.
pub async fn read_image_file(file: &web_sys::File, media_type: &str) -> Option<ImagePart> {
    let buf = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
        .await
        .ok()?;
    let bytes = js_sys::Uint8Array::new(&buf).to_vec();
    ImagePart::from_bytes(media_type, &bytes)
}

/// Smallest possible call, used by the "Test key" button.
pub async fn test_key(api_key: &str) -> Result<(), ClaudeError> {
    send_message(api_key, "Reply with the single word: ok", None).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_and_max_tokens_never_yield_text() {
        let refusal = r#"{"content":[{"type":"text","text":"{\"name\":\"x\"}"}],"stop_reason":"refusal"}"#;
        assert!(matches!(
            parse_messages_response(refusal),
            Err(ClaudeError::Refusal)
        ));

        let truncated = r#"{"content":[{"type":"text","text":"{\"name\":\"x\""}],"stop_reason":"max_tokens"}"#;
        assert!(matches!(
            parse_messages_response(truncated),
            Err(ClaudeError::MaxTokens)
        ));
    }

    #[test]
    fn thinking_blocks_are_dropped_and_text_blocks_joined() {
        let body = r#"{"content":[
            {"type":"thinking","thinking":"hmm"},
            {"type":"text","text":"{\"a\":"},
            {"type":"text","text":"1}"}
        ],"stop_reason":"end_turn"}"#;
        assert_eq!(parse_messages_response(body).unwrap(), "{\"a\":1}");
    }

    #[test]
    fn empty_text_is_an_error_not_an_empty_import() {
        let body = r#"{"content":[{"type":"thinking","thinking":"hmm"}],"stop_reason":"end_turn"}"#;
        assert!(matches!(
            parse_messages_response(body),
            Err(ClaudeError::EmptyResponse)
        ));
    }

    #[test]
    fn request_body_sets_model_and_effort_and_omits_thinking() {
        let body = request_body("hello", None);
        assert_eq!(body["model"], CLAUDE_MODEL);
        assert_eq!(body["output_config"]["effort"], CLAUDE_EFFORT);
        assert!(body.get("thinking").is_none());
        assert_eq!(body["messages"][0]["content"][0]["type"], "text");
    }

    #[test]
    fn image_goes_before_the_prompt_text() {
        let img = ImagePart::from_bytes("image/png", b"not-really-a-png").unwrap();
        let body = request_body("read this", Some(&img));
        assert_eq!(body["messages"][0]["content"][0]["type"], "image");
        assert_eq!(
            body["messages"][0]["content"][0]["source"]["media_type"],
            "image/png"
        );
        assert_eq!(body["messages"][0]["content"][1]["type"], "text");
    }

    #[test]
    fn oversized_or_non_image_files_are_rejected_before_any_request() {
        assert!(ImagePart::from_bytes("application/pdf", b"%PDF").is_none());
        assert!(ImagePart::from_bytes("image/png", b"").is_none());
        let too_big = vec![0u8; MAX_IMAGE_BYTES + 1];
        assert!(ImagePart::from_bytes("image/png", &too_big).is_none());
    }

    /// The key must never be reachable from the synced state. `ClaudeConfig`
    /// is a standalone struct with its own storage entry; this pins that it
    /// isn't accidentally given a `SyncConfig`-shaped serialization.
    #[test]
    fn claude_config_serializes_alone() {
        let cfg = ClaudeConfig { api_key: "sk-ant-secret".into() };
        let json = serde_json::to_string(&cfg).unwrap();
        assert_eq!(json, r#"{"api_key":"sk-ant-secret"}"#);

        let sync = crate::sync::SyncConfig::default();
        let sync_json = serde_json::to_string(&sync).unwrap();
        assert!(!sync_json.contains("api_key"));
    }
}

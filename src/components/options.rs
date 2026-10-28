use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::{JsCast, JsValue};

use crate::app::{current_date, current_datetime, build_synced_state, AppState, View};
use crate::coach::{apply_vitals_to_goals, build_coach_packet, parse_workout_response, PacketInput};
use crate::csv_utils::download_file;
use crate::models::{FocusLevel, PrimaryGoal, EQUIPMENT_OPTIONS};
use crate::claude_api::{self, read_image_file, ClaudeConfig, ImagePart};
use crate::storage;
use crate::sync::{self, SyncConfig, fetch_state, push_state};

#[component]
pub fn OptionsView() -> impl IntoView {
    let state = expect_context::<AppState>();

    let saved = storage::load_sync_config();
    let token = RwSignal::new(saved.token);
    let repo = RwSignal::new(if saved.repo.is_empty() {
        "you/wholesome-swolesome-data".to_string()
    } else {
        saved.repo
    });
    let branch = RwSignal::new(if saved.branch.is_empty() {
        "main".to_string()
    } else {
        saved.branch
    });
    let path = RwSignal::new(if saved.path.is_empty() {
        "state.json".to_string()
    } else {
        saved.path
    });

    let show_help = RwSignal::new(false);
    let test_status: RwSignal<Option<String>> = RwSignal::new(None);
    let is_testing = RwSignal::new(false);
    let is_pulling = RwSignal::new(false);
    let is_pushing = RwSignal::new(false);

    let current_config = move || SyncConfig {
        token: token.get_untracked(),
        repo: repo.get_untracked(),
        branch: branch.get_untracked(),
        path: path.get_untracked(),
    };

    let save_config = move || storage::save_sync_config(&current_config());

    let test_connection = move |_| {
        save_config();
        let cfg = current_config().to_github_config();
        is_testing.set(true);
        test_status.set(None);
        spawn_local(async move {
            match fetch_state(&cfg).await {
                Ok(remote) => {
                    let short = &remote.sha[..7.min(remote.sha.len())];
                    test_status.set(Some(format!("✓ Connected — sha {short}")));
                    state.sync_sha.set(Some(remote.sha));
                }
                Err(e) => test_status.set(Some(format!("✗ {e}"))),
            }
            is_testing.set(false);
        });
    };

    let pull = move |_| {
        save_config();
        let cfg = current_config().to_github_config();
        is_pulling.set(true);
        spawn_local(async move {
            match fetch_state(&cfg).await {
                Ok(remote) => {
                    state.sync_sha.set(Some(remote.sha));
                    match remote.state.updated_at.as_deref() {
                        None => state.show_toast("Remote is empty — push first"),
                        Some(remote_ts) => {
                            // Suppress auto-push while we hydrate signals from remote.
                            // Reset after 3s so the debounce window clears first.
                            state.suppress_push.set(true);
                            state.goals.set(remote.state.goals);
                            state.scheduled_workouts.set(remote.state.scheduled_workouts);
                            state.history.set(remote.state.exercise_history);
                            state.session_drafts.set(remote.state.session_drafts);
                            state.custom_exercises.set(remote.state.custom_exercises);
                            storage::save_last_push_at(remote_ts);
                            state.last_synced_at.set(Some(remote_ts.to_string()));
                            state.show_toast("Pulled from GitHub ↓");
                            let cb = wasm_bindgen::closure::Closure::once(move || {
                                state.suppress_push.set(false);
                            });
                            if let Some(window) = web_sys::window() {
                                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                                    cb.as_ref().unchecked_ref::<js_sys::Function>(),
                                    3000,
                                );
                            }
                            cb.forget();
                        }
                    }
                }
                Err(e) => state.show_toast(format!("Pull failed: {e}")),
            }
            is_pulling.set(false);
        });
    };

    let push = move |_| {
        save_config();
        let cfg = current_config().to_github_config();
        let sha = state.sync_sha.get_untracked();
        let synced = build_synced_state(state);
        is_pushing.set(true);
        spawn_local(async move {
            let result = push_state(&cfg, &synced, sha.as_deref()).await;
            match result {
                Ok(new_sha) => {
                    state.sync_sha.set(Some(new_sha));
                    let ts = synced.updated_at.clone().unwrap_or_default();
                    storage::save_last_push_at(&ts);
                    state.last_synced_at.set(Some(ts));
                    state.show_toast("Pushed to GitHub ↑");
                }
                Err(sync::SyncError::Conflict) => {
                    state.show_toast("Conflict — try pulling first");
                }
                Err(e) => state.show_toast(format!("Push failed: {e}")),
            }
            is_pushing.set(false);
        });
    };

    // ── Optional Claude API key ──────────────────────────────────────────────
    // Its own localStorage entry; nothing here is ever folded into the synced
    // state. Save/Test key/Clear mirror the Sync card's shape.
    let claude_key = RwSignal::new(storage::load_claude_config().api_key);
    let claude_status: RwSignal<Option<String>> = RwSignal::new(None);
    let claude_testing = RwSignal::new(false);

    let save_claude_key = move |_| {
        let key = claude_key.get_untracked().trim().to_string();
        storage::save_claude_config(&ClaudeConfig { api_key: key.clone() });
        claude_status.set(Some(if key.is_empty() {
            "Key cleared.".to_string()
        } else {
            "✓ Saved on this device.".to_string()
        }));
        state.show_toast("Claude API key saved");
    };

    let clear_claude_key = move |_| {
        claude_key.set(String::new());
        storage::clear_claude_config();
        claude_status.set(Some("Key cleared.".into()));
        state.show_toast("Claude API key cleared");
    };

    let test_claude_key = move |_| {
        let key = claude_key.get_untracked().trim().to_string();
        if key.is_empty() {
            // Empty field: inline validation, no request.
            claude_status.set(Some("Paste a key first.".into()));
            return;
        }
        claude_testing.set(true);
        claude_status.set(None);
        spawn_local(async move {
            match claude_api::test_key(&key).await {
                Ok(()) => claude_status.set(Some("✓ Key works.".into())),
                Err(e) => claude_status.set(Some(format!("✗ {e}"))),
            }
            claude_testing.set(false);
        });
    };

    let clear_local = move |_| {
        if let Some(storage) = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
        {
            let _ = storage.clear();
            state.show_toast("Local data cleared — reload to reset");
        }
    };

    let fmt_ts = |ts: String| {
        let date = js_sys::Date::new(&JsValue::from_str(&ts));
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            date.get_full_year(),
            date.get_month() + 1,
            date.get_date(),
            date.get_hours(),
            date.get_minutes()
        )
    };

    view! {
        <div class="page">
            <div class="page-header">
                <button class="back-btn" on:click=move |_| state.navigate(View::History)>
                    "‹ Back"
                </button>
                <h1 class="page-title">"Options"</h1>
            </div>

            <GoalsEditor/>

            <CardioMobilityEditor/>

            <div class="card claude-key-card" style="margin-bottom:12px">
                <div class="fw-600" style="margin-bottom:4px">"Claude API key (optional)"</div>
                <div class="text-muted text-sm" style="margin-bottom:12px">
                    "Paste your own Anthropic API key to generate workouts and read screenshots \
                     without copy/paste. Stored only on this device; never synced."
                </div>

                <input
                    type="password"
                    class="input claude-key-input"
                    style="margin-bottom:10px"
                    placeholder="sk-ant-..."
                    prop:value=move || claude_key.get()
                    on:input=move |e| claude_key.set(event_target_value(&e))
                />

                {move || claude_status.get().map(|s| view! {
                    <div class="text-sm claude-key-status" style="margin-bottom:8px; word-break:break-word">{s}</div>
                })}

                <div style="display:flex; gap:8px">
                    <button class="btn btn-primary claude-save-btn" style="flex:1" on:click=save_claude_key>"Save"</button>
                    <button
                        class="btn btn-secondary claude-test-btn"
                        style="flex:1"
                        disabled=move || claude_testing.get()
                        on:click=test_claude_key
                    >
                        {move || if claude_testing.get() { "Testing…" } else { "Test key" }}
                    </button>
                    <button class="btn btn-secondary claude-clear-btn" style="flex:1" on:click=clear_claude_key>"Clear"</button>
                </div>
            </div>

            <div class="card" style="margin-bottom:12px">
                <div style="display:flex; align-items:center; justify-content:space-between; margin-bottom:4px">
                    <div class="fw-600">"Sync (GitHub)"</div>
                    <button
                        class="btn btn-ghost btn-sm"
                        style="padding:2px 8px; font-size:15px; color:var(--accent)"
                        on:click=move |_| show_help.update(|v| *v = !*v)
                    >
                        {move || if show_help.get() { "✕" } else { "?" }}
                    </button>
                </div>

                {move || show_help.get().then(|| view! {
                    <div style="background:var(--surface2); border:1px solid var(--border); border-radius:var(--radius); padding:14px; margin-bottom:12px">

                        // Step 1
                        <div style="display:flex; gap:10px; align-items:flex-start; margin-bottom:12px">
                            <div style="background:var(--accent); color:#0f0d1f; border-radius:50%; width:22px; height:22px; display:flex; align-items:center; justify-content:center; font-size:11px; font-weight:700; flex-shrink:0; margin-top:1px">"1"</div>
                            <div>
                                <div class="fw-600 text-sm" style="margin-bottom:2px">"Create the data repo"</div>
                                <div class="text-sm text-muted">
                                    "Make a private GitHub repo — the default name below ("
                                    <code style="background:var(--surface); padding:1px 5px; border-radius:4px; font-size:11px">"wholesome-swolesome-data"</code>
                                    ") is already filled in for you."
                                </div>
                            </div>
                        </div>

                        // Step 2
                        <div style="display:flex; gap:10px; align-items:flex-start; margin-bottom:12px">
                            <div style="background:var(--accent); color:#0f0d1f; border-radius:50%; width:22px; height:22px; display:flex; align-items:center; justify-content:center; font-size:11px; font-weight:700; flex-shrink:0; margin-top:1px">"2"</div>
                            <div>
                                <div class="fw-600 text-sm" style="margin-bottom:2px">"Generate a fine-grained token"</div>
                                <div class="text-sm text-muted" style="margin-bottom:6px">
                                    "Go to GitHub → Settings → Developer settings → Fine-grained tokens → Generate new token."
                                </div>
                                <div class="text-sm text-muted" style="margin-bottom:4px">"Set:"</div>
                                <ul style="list-style:none; display:flex; flex-direction:column; gap:4px; padding-left:4px">
                                    <li class="text-sm text-muted">"• Repository access → Only select repositories → your data repo"</li>
                                    <li class="text-sm text-muted">"• Permissions → Contents → " <span class="fw-600" style="color:var(--text)">"Read and write"</span></li>
                                    <li class="text-sm text-muted">"• Expiration → 1 year (or no expiry)"</li>
                                </ul>
                            </div>
                        </div>

                        // Step 3
                        <div style="display:flex; gap:10px; align-items:flex-start; margin-bottom:14px">
                            <div style="background:var(--accent); color:#0f0d1f; border-radius:50%; width:22px; height:22px; display:flex; align-items:center; justify-content:center; font-size:11px; font-weight:700; flex-shrink:0; margin-top:1px">"3"</div>
                            <div>
                                <div class="fw-600 text-sm" style="margin-bottom:2px">"Paste and connect"</div>
                                <div class="text-sm text-muted">
                                    "Copy the generated token, paste it in the field below, then tap "
                                    <span class="fw-600" style="color:var(--text)">"Test connection"</span>
                                    ". Hit "
                                    <span class="fw-600" style="color:var(--text)">"Push to GitHub"</span>
                                    " to back up your data."
                                </div>
                            </div>
                        </div>

                        // Restore tip
                        <div style="border-top:1px solid var(--border); padding-top:10px">
                            <div class="text-sm" style="color:var(--pink); font-weight:600; margin-bottom:2px">"🔑 After clearing browser data"</div>
                            <div class="text-sm text-muted">"Open Options, paste your token again, tap " <span class="fw-600" style="color:var(--text)">"Pull from GitHub"</span>". Your data comes right back."</div>
                        </div>

                        <a
                            href="https://github.com/settings/tokens?type=beta"
                            target="_blank"
                            rel="noopener noreferrer"
                            style="display:block; margin-top:12px; text-align:center; padding:9px; background:var(--surface); border:1px solid var(--border); border-radius:var(--radius); color:var(--accent); font-size:13px; font-weight:600; text-decoration:none"
                        >
                            "Open GitHub token settings ↗"
                        </a>
                    </div>
                })}

                <div class="text-muted text-sm" style="margin-bottom:12px">
                    "Backs up plan, history, drafts, and custom exercises to a private repo. \
                     Clearing local data won't lose anything — paste your PAT again and pull."
                </div>

                <label class="text-sm text-muted">"Personal access token"</label>
                <input
                    type="password"
                    class="input"
                    style="margin-bottom:10px"
                    placeholder="github_pat_..."
                    prop:value=move || token.get()
                    on:input=move |e| token.set(event_target_value(&e))
                />

                <label class="text-sm text-muted">"Repo (owner/name)"</label>
                <input
                    type="text"
                    class="input"
                    style="margin-bottom:10px"
                    prop:value=move || repo.get()
                    on:input=move |e| repo.set(event_target_value(&e))
                />

                <div style="display:flex; gap:8px; margin-bottom:12px">
                    <div style="flex:1">
                        <label class="text-sm text-muted">"Branch"</label>
                        <input
                            type="text"
                            class="input"
                            prop:value=move || branch.get()
                            on:input=move |e| branch.set(event_target_value(&e))
                        />
                    </div>
                    <div style="flex:1">
                        <label class="text-sm text-muted">"Path"</label>
                        <input
                            type="text"
                            class="input"
                            prop:value=move || path.get()
                            on:input=move |e| path.set(event_target_value(&e))
                        />
                    </div>
                </div>

                {move || test_status.get().map(|s| view! {
                    <div class="text-sm" style="margin-bottom:8px; word-break:break-all">{s}</div>
                })}

                <button
                    class="btn btn-secondary btn-full"
                    disabled=move || is_testing.get()
                    on:click=test_connection
                >
                    {move || if is_testing.get() { "Testing…" } else { "Test connection" }}
                </button>
            </div>

            <div class="card" style="margin-bottom:12px">
                <div class="fw-600" style="margin-bottom:4px">"Sync actions"</div>
                <div class="text-muted text-sm" style="margin-bottom:12px">
                    "Last synced: "
                    <span class="fw-600">
                        {move || state.last_synced_at.get()
                            .map(fmt_ts)
                            .unwrap_or_else(|| "Never".to_string())}
                    </span>
                </div>
                <div style="display:flex; gap:8px">
                    <button
                        class="btn btn-secondary"
                        style="flex:1"
                        disabled=move || is_pulling.get() || is_pushing.get()
                        on:click=pull
                    >
                        {move || if is_pulling.get() { "Pulling…" } else { "Pull from GitHub" }}
                    </button>
                    <button
                        class="btn btn-primary"
                        style="flex:1"
                        disabled=move || is_pushing.get() || is_pulling.get()
                        on:click=push
                    >
                        {move || if is_pushing.get() { "Pushing…" } else { "Push to GitHub" }}
                    </button>
                </div>
            </div>

            <div class="card">
                <div class="fw-600" style="margin-bottom:4px; color: var(--danger, #c4302b)">
                    "Danger zone"
                </div>
                <div class="text-muted text-sm" style="margin-bottom:12px">
                    "Wipes all local data on this device. If sync is configured and you've \
                     pushed recently, your data lives in GitHub and you can pull it back."
                </div>
                <button class="btn btn-danger btn-full" on:click=clear_local>
                    "Clear local data"
                </button>
            </div>
        </div>
    }
}

// ── Goals editor ─────────────────────────────────────────────────────────────

/// Which large free-text goal field the full-screen editor is editing.
#[derive(Clone, Copy, PartialEq)]
enum NoteField {
    Avoid,
    Notes,
}

impl NoteField {
    fn title(self) -> &'static str {
        match self {
            NoteField::Avoid => "Injuries / lifts to avoid",
            NoteField::Notes => "Notes for the coach",
        }
    }
    fn placeholder(self) -> &'static str {
        match self {
            NoteField::Avoid => "e.g. left shoulder impingement; no overhead press",
            NoteField::Notes => "e.g. prefer compound lifts; bias glutes; warm-up included separately",
        }
    }
}

#[component]
fn GoalsEditor() -> impl IntoView {
    let state = expect_context::<AppState>();

    let goal_set = move |g: PrimaryGoal| {
        state.goals.update(|x| x.primary_goal = g);
    };
    let set_sessions = move |v: u32| state.goals.update(|x| x.sessions_per_week = v);
    let set_minutes = move |v: u32| state.goals.update(|x| x.session_minutes = v);

    // Full-screen note editor: tap a preview card to open a top-anchored
    // overlay that edits a local `draft`; Done commits, Cancel discards. The
    // commit writes back to `goals`, which the existing localStorage + push
    // effects persist automatically (no new save plumbing).
    let editing: RwSignal<Option<NoteField>> = RwSignal::new(None);
    let draft = RwSignal::new(String::new());

    let open_editor = move |field: NoteField| {
        let current = match field {
            NoteField::Avoid => state.goals.get_untracked().avoid,
            NoteField::Notes => state.goals.get_untracked().notes,
        };
        draft.set(current);
        editing.set(Some(field));

        // Focus the textarea once it mounts so the keyboard opens immediately.
        // Same set_timeout + query_selector idiom used in exercises.rs.
        if let Some(window) = web_sys::window() {
            let cb = wasm_bindgen::closure::Closure::once(move || {
                if let Some(el) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.query_selector(".note-editor-area").ok().flatten())
                {
                    let _ = el.unchecked_into::<web_sys::HtmlElement>().focus();
                }
            });
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                wasm_bindgen::JsCast::unchecked_ref::<js_sys::Function>(cb.as_ref()),
                50,
            );
            cb.forget();
        }
    };

    let commit = move |field: NoteField| {
        let text = draft.get_untracked();
        state.goals.update(|g| match field {
            NoteField::Avoid => g.avoid = text,
            NoteField::Notes => g.notes = text,
        });
        editing.set(None);
    };

    let toggle_equipment = move |eq: String| {
        state.goals.update(|x| {
            if let Some(p) = x.equipment.iter().position(|s| s == &eq) {
                x.equipment.remove(p);
            } else {
                x.equipment.push(eq);
            }
        });
    };

    view! {
        <div class="card" style="margin-bottom:12px">
            <div class="fw-600" style="margin-bottom:8px">"Training goals"</div>
            <div class="text-muted text-sm" style="margin-bottom:10px">
                "The coach uses these to plan your next workout."
            </div>

            <label class="text-sm text-muted">"Primary goal"</label>
            <div style="margin-bottom:14px">
                {PrimaryGoal::all().iter().copied().map(|g| {
                    let active = move || state.goals.get().primary_goal == g;
                    view! {
                        <span
                            class="goal-pill"
                            class:active=active
                            on:click=move |_| goal_set(g)
                        >{g.label()}</span>
                    }
                }).collect_view()}
            </div>

            <div style="display:flex; gap:8px; margin-bottom:14px">
                <div style="flex:1">
                    <label class="text-sm text-muted">"Sessions / week"</label>
                    <input
                        type="number"
                        min="1" max="7"
                        class="input"
                        prop:value=move || state.goals.get().sessions_per_week.to_string()
                        on:change=move |e| set_sessions(event_target_value(&e).parse().unwrap_or(4))
                    />
                </div>
                <div style="flex:1">
                    <label class="text-sm text-muted">"Session minutes"</label>
                    <input
                        type="number"
                        min="15" max="240" step="5"
                        class="input"
                        prop:value=move || state.goals.get().session_minutes.to_string()
                        on:change=move |e| set_minutes(event_target_value(&e).parse().unwrap_or(60))
                    />
                </div>
            </div>

            <label class="text-sm text-muted">"Available equipment"</label>
            <div style="margin-bottom:14px">
                {EQUIPMENT_OPTIONS.iter().copied().map(|eq| {
                    let eq_s = eq.to_string();
                    let eq_for_active = eq_s.clone();
                    let eq_for_click = eq_s.clone();
                    let active = move || state.goals.get().equipment.iter().any(|s| s == &eq_for_active);
                    view! {
                        <span
                            class="goal-pill"
                            class:active=active
                            on:click=move |_| toggle_equipment(eq_for_click.clone())
                        >{eq.to_string()}</span>
                    }
                }).collect_view()}
            </div>
            <div class="text-muted text-sm" style="margin-bottom:14px; margin-top:-8px">
                "Tip: leave all unselected = assume full commercial gym."
            </div>

            <label class="text-sm text-muted">"Injuries / lifts to avoid"</label>
            <div
                class="note-preview"
                class=("is-empty", move || state.goals.get().avoid.trim().is_empty())
                style="margin-bottom:10px"
                on:click=move |_| open_editor(NoteField::Avoid)
            >
                {move || {
                    let v = state.goals.get().avoid;
                    if v.trim().is_empty() {
                        "Tap to add injuries / lifts to avoid…".to_string()
                    } else {
                        v
                    }
                }}
            </div>

            <label class="text-sm text-muted">"Notes for the coach"</label>
            <div
                class="note-preview"
                class=("is-empty", move || state.goals.get().notes.trim().is_empty())
                on:click=move |_| open_editor(NoteField::Notes)
            >
                {move || {
                    let v = state.goals.get().notes;
                    if v.trim().is_empty() {
                        "Tap to add notes for the coach…".to_string()
                    } else {
                        v
                    }
                }}
            </div>
        </div>

        {move || editing.get().map(|field| view! {
            <div class="note-editor">
                <div class="note-editor-bar">
                    <button
                        class="btn btn-ghost"
                        on:click=move |_| editing.set(None)
                    >"Cancel"</button>
                    <span class="note-editor-title">{field.title()}</span>
                    <button
                        class="btn btn-primary btn-sm"
                        on:click=move |_| commit(field)
                    >"Done"</button>
                </div>
                <textarea
                    class="note-editor-area"
                    placeholder=field.placeholder()
                    prop:value=move || draft.get()
                    on:input=move |e| draft.set(event_target_value(&e))
                />
            </div>
        })}
    }
}

// ── Cardio & mobility editor ─────────────────────────────────────────────────

#[component]
fn CardioMobilityEditor() -> impl IntoView {
    let state = expect_context::<AppState>();

    let set_weekly_cardio = move |raw: String| {
        let parsed: Option<u32> = if raw.trim().is_empty() {
            None
        } else {
            raw.trim().parse().ok().or_else(|| state.goals.get_untracked().weekly_cardio_minutes_target)
        };
        state.goals.update(|g| g.weekly_cardio_minutes_target = parsed);
    };

    let set_vo2 = move |raw: String| {
        let raw = raw.trim().to_string();
        if raw.is_empty() {
            state.goals.update(|g| {
                g.vo2_max_latest = None;
                g.vo2_max_updated = None;
            });
            return;
        }
        if let Ok(v) = raw.parse::<f32>() {
            let today = current_date();
            state.goals.update(|g| {
                g.vo2_max_latest = Some(v);
                g.vo2_max_updated = Some(today.clone());
            });
        }
    };

    let set_mobility = move |f: FocusLevel| state.goals.update(|g| g.mobility_focus = f);
    let set_balance = move |f: FocusLevel| state.goals.update(|g| g.balance_focus = f);

    view! {
        <div class="card" style="margin-bottom:12px">
            <div class="fw-600" style="margin-bottom:8px">"Cardio & mobility"</div>
            <div class="text-muted text-sm" style="margin-bottom:10px">
                "Optional. The coach plans cardio, mobility, and balance work against these."
            </div>

            <div style="display:flex; gap:8px; margin-bottom:14px">
                <div style="flex:1">
                    <label class="text-sm text-muted">"Weekly cardio minutes (target)"</label>
                    <input
                        type="number"
                        min="0" max="600" step="5"
                        class="input"
                        placeholder="empty = no target"
                        prop:value=move || state.goals.get().weekly_cardio_minutes_target
                            .map(|n| n.to_string()).unwrap_or_default()
                        on:change=move |e| set_weekly_cardio(event_target_value(&e))
                    />
                </div>
                <div style="flex:1">
                    <label class="text-sm text-muted">"VO2 max (latest)"</label>
                    <input
                        type="number"
                        min="0" max="100" step="0.1"
                        class="input"
                        placeholder="e.g. 36.4"
                        prop:value=move || state.goals.get().vo2_max_latest
                            .map(|v| if v.fract() == 0.0 { format!("{:.0}", v) } else { format!("{:.1}", v) })
                            .unwrap_or_default()
                        on:change=move |e| set_vo2(event_target_value(&e))
                    />
                    <div class="text-muted text-sm" style="margin-top:4px">
                        {move || match state.goals.get().vo2_max_updated {
                            Some(d) => format!("Updated: {d}"),
                            None => "Updated: —".to_string(),
                        }}
                    </div>
                </div>
            </div>

            <label class="text-sm text-muted">"Mobility focus"</label>
            <div style="margin-bottom:14px">
                {FocusLevel::all().iter().copied().map(|f| {
                    let active = move || state.goals.get().mobility_focus == f;
                    view! {
                        <span
                            class="goal-pill"
                            class:active=active
                            on:click=move |_| set_mobility(f)
                        >{f.label()}</span>
                    }
                }).collect_view()}
            </div>

            <label class="text-sm text-muted">"Balance focus"</label>
            <div>
                {FocusLevel::all().iter().copied().map(|f| {
                    let active = move || state.goals.get().balance_focus == f;
                    view! {
                        <span
                            class="goal-pill"
                            class:active=active
                            on:click=move |_| set_balance(f)
                        >{f.label()}</span>
                    }
                }).collect_view()}
            </div>

            <div class="text-muted text-sm" style="margin-top:10px">
                "Tip: paste an Apple Health VO2 max screenshot into Claude alongside the Coach Brief — Claude will extract the value into the import response."
            </div>
        </div>
    }
}

// ── Coach packet view ────────────────────────────────────────────────────────

#[component]
pub fn CoachPacketView() -> impl IntoView {
    let state = expect_context::<AppState>();

    let target_date = move || {
        let today = current_date();
        let has_today = state
            .scheduled_workouts
            .get()
            .iter()
            .any(|w| w.date == today);
        if has_today { tomorrow() } else { today }
    };
    let packet = move || {
        let goals = state.goals.get();
        let history = state.history.get();
        let library = state.library.get();
        let scheduled = state.scheduled_workouts.get();
        let today = current_date();
        let target = target_date();
        build_coach_packet(PacketInput {
            goals: &goals,
            history: &history,
            library: &library,
            scheduled: &scheduled,
            today: &today,
            target_date: &target,
        })
    };

    let response_text: RwSignal<String> = RwSignal::new(String::new());
    let import_status: RwSignal<Option<String>> = RwSignal::new(None);
    // The brief is read-on-demand: Copy and the JSON paste box are the actions,
    // so the packet itself lives behind a full-screen viewer (#42's overlay
    // pattern) instead of sitting between them. Open flag only — no new state.
    let viewing_brief = RwSignal::new(false);

    let copy_packet = move |_| {
        let text = packet();
        let window = web_sys::window();
        if let Some(window) = window {
            let _ = window.navigator().clipboard().write_text(&text);
            state.show_toast("Copied to clipboard");
        }
    };
    let download_packet = move |_| {
        let text = packet();
        download_file("coach_brief.md", &text);
    };

    // The one import path. Both the pasted response and the API response land
    // here, so a given JSON blob produces the same schedule either way.
    let apply_response = move |text: String| {
        let target = target_date();
        let created = current_datetime();
        let library = state.library.get_untracked();
        match parse_workout_response(&text, &target, &created, &library) {
            Ok(parsed) => {
                let label = parsed.workout.name.clone();
                state.scheduled_workouts.update(|v| {
                    v.retain(|w| w.date != target);
                    v.push(parsed.workout);
                });
                let mut vitals_msg = String::new();
                if let Some(v) = parsed.vitals {
                    let applied = {
                        let mut applied = false;
                        state.goals.update(|g| {
                            applied = apply_vitals_to_goals(&v, g);
                        });
                        applied
                    };
                    if applied {
                        vitals_msg = format!(" · VO2 max → {:.1} ({})", v.vo2_max, v.source_date);
                    }
                }
                import_status.set(Some(format!("✓ Imported '{label}' for {target}{vitals_msg}")));
                response_text.set(String::new());
                state.show_toast("Workout added");
            }
            Err(e) => import_status.set(Some(format!("✗ {e}"))),
        }
    };

    let import_workout = move |_| {
        let text = response_text.get_untracked();
        if text.trim().is_empty() {
            import_status.set(Some("Paste Claude's JSON response first.".into()));
            return;
        }
        apply_response(text);
    };

    // ── Optional direct-to-API path ──────────────────────────────────────────
    // Only rendered when a key is saved. The copy/paste UI below stays put
    // either way, so a failed call leaves the user exactly where they were.
    let claude_key = RwSignal::new(storage::load_claude_config().api_key);
    let has_key = move || !claude_key.get().trim().is_empty();
    let generating = RwSignal::new(false);
    // Optional Apple Health VO2 max screenshot to send with the brief: the
    // `vitals` block comes back in the same response the workout does.
    let vitals_image: RwSignal<Option<ImagePart>> = RwSignal::new(None);
    let vitals_name: RwSignal<Option<String>> = RwSignal::new(None);

    let pick_vitals_image = move |ev: leptos::ev::Event| {
        let input = event_target::<web_sys::HtmlInputElement>(&ev);
        let Some(file) = input.files().and_then(|f| f.get(0)) else { return };
        let name = file.name();
        let media_type = file.type_();
        spawn_local(async move {
            match read_image_file(&file, &media_type).await {
                Some(part) => {
                    vitals_image.set(Some(part));
                    vitals_name.set(Some(name));
                }
                None => {
                    vitals_image.set(None);
                    vitals_name.set(None);
                    import_status.set(Some(
                        "✗ That file isn't a usable image (wrong type, empty, or over 5 MB).".into(),
                    ));
                }
            }
        });
    };

    let generate_with_claude = move |_| {
        let key = claude_key.get_untracked().trim().to_string();
        if key.is_empty() {
            import_status.set(Some("No Claude API key saved — add one in Options.".into()));
            return;
        }
        let prompt = packet();
        let image = vitals_image.get_untracked();
        generating.set(true);
        import_status.set(None);
        spawn_local(async move {
            // A key cleared mid-flight doesn't cancel this call: `key` was
            // captured before the await, and the response still imports.
            let result = claude_api::send_message(&key, &prompt, image.as_ref()).await;
            generating.set(false);
            match result {
                Ok(text) => apply_response(text),
                Err(e) => {
                    import_status.set(Some(format!("✗ {e}")));
                    state.show_toast(format!("{e}"));
                }
            }
        });
    };

    view! {
        <div class="page">
            <div class="page-header">
                <button class="back-btn" on:click=move |_| state.navigate(View::Home)>"‹ Back"</button>
                <h1 class="page-title">"Coach Brief"</h1>
            </div>

            <div class="card" style="margin-bottom:12px">
                <div class="text-sm text-muted">
                    "Target workout date: " <span class="fw-600">{target_date}</span>
                </div>
                <div class="text-muted text-sm" style="margin-top:8px">
                    "1) Copy this brief → 2) paste into Claude Code (any session) → 3) paste Claude's JSON response below → 4) Import."
                </div>
            </div>

            {move || has_key().then(|| view! {
                <div class="card claude-generate-card" style="margin-bottom:12px">
                    <div class="fw-600" style="margin-bottom:6px">"Generate with your Claude key"</div>
                    <div class="text-muted text-sm" style="margin-bottom:10px">
                        "Sends this brief straight to Claude and schedules the workout it returns. \
                         Copy/paste below still works if the call fails."
                    </div>
                    <label class="text-sm text-muted">"Apple Health VO2 max screenshot (optional)"</label>
                    <input
                        type="file"
                        accept="image/*"
                        class="claude-vitals-file"
                        style="margin-bottom:10px"
                        on:change=pick_vitals_image
                    />
                    {move || vitals_name.get().map(|n| view! {
                        <div class="text-sm text-muted" style="margin-bottom:8px">"Attached: " {n}</div>
                    })}
                    <button
                        class="btn btn-primary btn-full claude-generate-btn"
                        disabled=move || generating.get()
                        on:click=generate_with_claude
                    >
                        {move || if generating.get() { "Generating…" } else { "✨ Generate with Claude" }}
                    </button>
                </div>
            })}

            // Copy keeps its own full-width row: it's the dominant action, and it
            // must work without opening the viewer first.
            <button
                class="btn btn-primary btn-full brief-copy-btn"
                style="margin-bottom:8px"
                on:click=copy_packet
            >"📋 Copy brief"</button>

            <div style="display:flex; gap:8px; margin-bottom:8px">
                <button
                    class="btn btn-secondary brief-view-btn"
                    style="flex:1"
                    on:click=move |_| viewing_brief.set(true)
                >"👁 View brief"</button>
                <button
                    class="btn btn-secondary brief-download-btn"
                    style="flex:1"
                    on:click=download_packet
                >"⬇ Download .md"</button>
            </div>

            <div class="card" style="margin-top:14px">
                <div class="fw-600" style="margin-bottom:6px">"Paste Claude's JSON response"</div>
                <textarea
                    class="input"
                    rows="6"
                    style="font-family: ui-monospace, monospace; font-size: 11px"
                    placeholder="{ &quot;name&quot;: ..., &quot;exercises&quot;: [...] }"
                    prop:value=move || response_text.get()
                    on:input=move |e| response_text.set(event_target_value(&e))
                />
                {move || import_status.get().map(|s| view! {
                    <div class="text-sm" style="margin-top:8px">{s}</div>
                })}
                <button
                    class="btn btn-primary btn-full"
                    style="margin-top:8px"
                    on:click=import_workout
                >"Import workout"</button>
            </div>
        </div>

        // Read-only brief viewer. Same overlay chrome as the note editor (#42),
        // but the body is the packet in a scrolling <pre> — nothing to edit, and
        // Copy is repeated in the bar so a reader needn't close first.
        {move || viewing_brief.get().then(|| view! {
            <div class="note-editor brief-viewer">
                <div class="note-editor-bar">
                    <button
                        class="btn btn-ghost brief-viewer-close"
                        on:click=move |_| viewing_brief.set(false)
                    >"✕ Close"</button>
                    <span class="note-editor-title">"Coach Brief"</span>
                    <button
                        class="btn btn-secondary btn-sm brief-viewer-copy"
                        on:click=copy_packet
                    >"📋 Copy"</button>
                </div>
                <pre class="coach-packet-pre coach-packet-viewer">{packet}</pre>
            </div>
        })}
    }
}

fn tomorrow() -> String {
    let d = js_sys::Date::new_0();
    d.set_date(d.get_date() + 1);
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date(),
    )
}

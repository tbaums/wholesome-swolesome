use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::{JsCast, JsValue};

use crate::app::{AppState, View, build_synced_state};
use crate::storage;
use crate::sync::{self, SyncConfig, fetch_state, push_state};

#[component]
pub fn OptionsView() -> impl IntoView {
    let state = expect_context::<AppState>();

    let saved = storage::load_sync_config();
    let token = RwSignal::new(saved.token);
    let repo = RwSignal::new(if saved.repo.is_empty() {
        "tbaums/wholesome-swolesome-data".to_string()
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
                            if let Some(plan) = remote.state.plan {
                                state.plan.set(plan);
                            }
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

            <div class="card" style="margin-bottom:12px">
                <div class="fw-600" style="margin-bottom:4px">"Sync (GitHub)"</div>
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

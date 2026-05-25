use leptos::prelude::*;

use crate::app::{AppState, View};
use crate::models::{muscle_label, LibraryExercise, ALL_MUSCLES};

// ── Browse view ──────────────────────────────────────────────────────────────

#[component]
pub fn LibraryView() -> impl IntoView {
    let state = expect_context::<AppState>();
    let query: RwSignal<String> = RwSignal::new(String::new());
    let muscle_filter: RwSignal<Option<String>> = RwSignal::new(None);
    let equip_filter: RwSignal<Option<String>> = RwSignal::new(None);
    let cat_filter: RwSignal<Option<String>> = RwSignal::new(None);

    let filtered = move || -> Vec<LibraryExercise> {
        let lib = state.library.get();
        let q = query.get().to_lowercase();
        let m = muscle_filter.get();
        let eq = equip_filter.get();
        let cat = cat_filter.get();
        lib.into_iter()
            .filter(|e| {
                if !q.is_empty() && !e.name.to_lowercase().contains(&q) {
                    return false;
                }
                if let Some(m) = m.as_deref() {
                    if !e.primary_muscles.iter().any(|p| p == m)
                        && !e.secondary_muscles.iter().any(|p| p == m)
                    {
                        return false;
                    }
                }
                if let Some(eq) = eq.as_deref() {
                    if e.equipment.as_deref() != Some(eq) {
                        return false;
                    }
                }
                if let Some(cat) = cat.as_deref() {
                    if e.category != cat {
                        return false;
                    }
                }
                true
            })
            .collect()
    };

    let categories = ["strength", "cardio", "plyometrics", "stretching"];
    let equipments = ["barbell", "dumbbell", "machine", "cable", "body only", "kettlebells"];

    view! {
        <div class="page">
            <div class="page-header">
                <h1 class="page-title">"Exercise Library"</h1>
            </div>

            <input
                type="text"
                class="input library-search"
                placeholder="Search exercises…"
                prop:value=move || query.get()
                on:input=move |e| query.set(event_target_value(&e))
            />

            // Muscle chips
            <div class="filter-chips">
                <button
                    class="filter-chip"
                    class:active=move || muscle_filter.get().is_none()
                    on:click=move |_| muscle_filter.set(None)
                >"All muscles"</button>
                {ALL_MUSCLES.iter().map(|m| {
                    let m_str = (*m).to_string();
                    let m_for_active = m_str.clone();
                    let m_for_click = m_str.clone();
                    view! {
                        <button
                            class="filter-chip"
                            class:active=move || muscle_filter.get().as_deref() == Some(m_for_active.as_str())
                            on:click=move |_| muscle_filter.set(Some(m_for_click.clone()))
                        >{muscle_label(m).to_string()}</button>
                    }
                }).collect_view()}
            </div>

            // Category chips
            <div class="filter-chips">
                <button
                    class="filter-chip"
                    class:active=move || cat_filter.get().is_none()
                    on:click=move |_| cat_filter.set(None)
                >"All types"</button>
                {categories.iter().map(|c| {
                    let c_str = (*c).to_string();
                    let c_for_active = c_str.clone();
                    let c_for_click = c_str.clone();
                    view! {
                        <button
                            class="filter-chip"
                            class:active=move || cat_filter.get().as_deref() == Some(c_for_active.as_str())
                            on:click=move |_| cat_filter.set(Some(c_for_click.clone()))
                        >{c_str.clone()}</button>
                    }
                }).collect_view()}
            </div>

            // Equipment chips
            <div class="filter-chips">
                <button
                    class="filter-chip"
                    class:active=move || equip_filter.get().is_none()
                    on:click=move |_| equip_filter.set(None)
                >"Any equipment"</button>
                {equipments.iter().map(|eq| {
                    let eq_str = (*eq).to_string();
                    let eq_for_active = eq_str.clone();
                    let eq_for_click = eq_str.clone();
                    view! {
                        <button
                            class="filter-chip"
                            class:active=move || equip_filter.get().as_deref() == Some(eq_for_active.as_str())
                            on:click=move |_| equip_filter.set(Some(eq_for_click.clone()))
                        >{eq_str.clone()}</button>
                    }
                }).collect_view()}
            </div>

            // Results
            {move || {
                let list = filtered();
                let count = list.len();
                view! {
                    <div class="text-muted text-sm" style="margin-bottom:6px">
                        {count} " exercises"
                    </div>
                    <div>
                        {list.into_iter().take(120).map(|e| view! { <LibraryItem ex=e/> }).collect_view()}
                    </div>
                }
            }}
        </div>
    }
}

#[component]
fn LibraryItem(ex: LibraryExercise) -> impl IntoView {
    let state = expect_context::<AppState>();
    let id = ex.id.clone();
    let name = ex.name.clone();
    let thumb = ex
        .images
        .first()
        .map(|p| format!("data/exercises/{}", p))
        .unwrap_or_default();
    let meta_parts: Vec<String> = ex
        .primary_muscles
        .iter()
        .map(|m| muscle_label(m).to_string())
        .collect();
    let meta = meta_parts.join(", ");
    let equip = ex.equipment.clone().unwrap_or_default();

    let on_click = move |_| state.navigate(View::LibraryDetail { exercise_id: id.clone(), from: None });

    view! {
        <div class="library-item" on:click=on_click>
            {(!thumb.is_empty()).then(|| view! { <img class="library-thumb" src=thumb.clone() loading="lazy"/> })}
            <div>
                <div class="library-item-name">{name}</div>
                <div class="library-item-meta">{meta} " · " {equip}</div>
            </div>
        </div>
    }
}

// ── Detail view ──────────────────────────────────────────────────────────────

#[component]
pub fn LibraryDetailView(
    exercise_id: String,
    from: Option<Box<View>>,
) -> impl IntoView {
    let state = expect_context::<AppState>();
    let id = exercise_id.clone();

    let back_view = from.map(|v| *v).unwrap_or(View::Library);
    let back_label = match &back_view {
        View::Library => "‹ Library",
        _ => "‹ Back",
    };

    let entry = move || -> Option<LibraryExercise> {
        let id = id.clone();
        state.library.get().into_iter().find(|e| e.id == id)
    };

    view! {
        <div class="page">
            <div class="page-header">
                <button class="back-btn" on:click=move |_| state.navigate(back_view.clone())>{back_label}</button>
            </div>
            {move || match entry() {
                None => view! {
                    <div class="empty">
                        <div class="empty-icon">"🤔"</div>
                        <div>"Exercise not found."</div>
                    </div>
                }.into_any(),
                Some(e) => {
                    let images = e.images.clone();
                    let name = e.name.clone();
                    let primary = e.primary_muscles.clone();
                    let secondary = e.secondary_muscles.clone();
                    let primary_for_tags = primary.clone();
                    let secondary_for_tags = secondary.clone();
                    let instructions = e.instructions.clone();
                    let equipment = e.equipment.clone().unwrap_or_else(|| "—".into());
                    let force = e.force.clone().unwrap_or_else(|| "—".into());
                    let level = e.level.clone();
                    let mechanic = e.mechanic.clone().unwrap_or_else(|| "—".into());
                    let category = e.category.clone();

                    view! {
                        <div class="page-title" style="margin-bottom:8px">{name}</div>

                        <div class="lib-detail-images">
                            {images.into_iter().take(2).map(|p| view! {
                                <img class="lib-detail-img" src=format!("data/exercises/{}", p) loading="lazy"/>
                            }).collect_view()}
                        </div>

                        <div style="margin-bottom:12px">
                            {primary_for_tags.iter().map(|m| view! {
                                <span class="lib-tag primary">{muscle_label(m).to_string()}</span>
                            }).collect_view()}
                            {secondary_for_tags.iter().map(|m| view! {
                                <span class="lib-tag">{muscle_label(m).to_string()}</span>
                            }).collect_view()}
                        </div>

                        <crate::components::body_heatmap::BodyMuscleHighlight
                            primary=primary
                            secondary=secondary
                        />

                        <div class="card" style="margin-top:12px">
                            <div class="card-sub">
                                "Category: " <b>{category}</b> " · Level: " <b>{level}</b>
                            </div>
                            <div class="card-sub" style="margin-top:4px">
                                "Equipment: " <b>{equipment}</b> " · Force: " <b>{force}</b> " · Mechanic: " <b>{mechanic}</b>
                            </div>
                        </div>

                        <div class="lib-detail-instructions">
                            <div class="fw-600" style="margin-bottom:6px">"How to do it"</div>
                            <ol>
                                {instructions.into_iter().map(|s| view! { <li>{s}</li> }).collect_view()}
                            </ol>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

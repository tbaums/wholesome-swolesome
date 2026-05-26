//! Front + back human silhouette with per-muscle coloring.
//!
//! Two use modes:
//!   - `BodyHeatmap`: colors each muscle region by recency since last worked,
//!     derived from history × library lookups.
//!   - `BodyMuscleHighlight`: highlights a fixed set of muscles (primary +
//!     secondary), used in the library detail view.

use leptos::prelude::*;

use crate::app::{current_date, AppState};
use crate::library::{days_between, last_hit_by_muscle, recency_bucket, RecencyBucket};
use crate::models::ALL_MUSCLES;

// ── Public components ─────────────────────────────────────────────────────────

#[component]
pub fn BodyHeatmap() -> impl IntoView {
    let state = expect_context::<AppState>();

    let muscle_fills = move || {
        let history = state.history.get();
        let library = state.library.get();
        let last = last_hit_by_muscle(&history, &library);
        let today = current_date();
        let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for m in ALL_MUSCLES {
            let bucket = last
                .get(*m)
                .and_then(|d| days_between(d, &today))
                .and_then(recency_bucket)
                .unwrap_or(RecencyBucket::Stale);
            map.insert((*m).to_string(), bucket.color().to_string());
        }
        map
    };

    view! {
        <div class="heatmap-wrap">
            <div class="heatmap-row">
                {move || render_front(&muscle_fills())}
                {move || render_back(&muscle_fills())}
            </div>
            <HeatmapLegend/>
        </div>
    }
}

/// For library detail view: highlight primary muscles in pink, secondary in
/// lighter pink, rest in muted gray.
#[component]
pub fn BodyMuscleHighlight(
    #[prop()] primary: Vec<String>,
    #[prop()] secondary: Vec<String>,
) -> impl IntoView {
    let fills = {
        let primary: std::collections::HashSet<_> =
            primary.iter().map(|s| s.to_lowercase()).collect();
        let secondary: std::collections::HashSet<_> =
            secondary.iter().map(|s| s.to_lowercase()).collect();
        let mut map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for m in ALL_MUSCLES {
            let k = m.to_lowercase();
            let fill = if primary.contains(&k) {
                "#F5A9B8".to_string() // bright pink
            } else if secondary.contains(&k) {
                "#7c3958".to_string() // muted pink
            } else {
                "#2d2050".to_string() // body base
            };
            map.insert((*m).to_string(), fill);
        }
        map
    };

    view! {
        <div class="heatmap-wrap">
            <div class="heatmap-row">
                {render_front(&fills)}
                {render_back(&fills)}
            </div>
        </div>
    }
}

#[component]
fn HeatmapLegend() -> impl IntoView {
    let items = [
        RecencyBucket::Recent,
        RecencyBucket::Week,
        RecencyBucket::TwoWeeks,
        RecencyBucket::Stale,
    ];
    view! {
        <div class="heatmap-legend">
            {items.into_iter().map(|b| view! {
                <div class="legend-item">
                    <span class="legend-swatch" style=format!("background:{}", b.color())/>
                    <span class="legend-label">{b.label()}</span>
                </div>
            }).collect_view()}
        </div>
    }
}

// ── SVG construction ─────────────────────────────────────────────────────────
//
// viewBox 260 x 480 per side (content in 0..200 x 0..440 with padding).
// neck, torso, arms, legs. Muscle regions are overlaid paths fitted to those
// shapes. Coordinates are hand-authored — not perfect, but readable on a
// phone and clearly partition the body into the ~15 muscle zones we track.

type Fills = std::collections::HashMap<String, String>;

fn fill_of(fills: &Fills, muscle: &str) -> String {
    fills
        .get(muscle)
        .cloned()
        .unwrap_or_else(|| "#2d2050".to_string())
}

fn render_front(fills: &Fills) -> impl IntoView {
    let outline = "#3d2878";

    view! {
        <svg class="body-svg" viewBox="-30 -10 260 480" xmlns="http://www.w3.org/2000/svg">
            // ── Silhouette base ─────────────────────────────────────────
            // Head + neck
            <circle cx="100" cy="30" r="22" fill="#2d2050" stroke=outline stroke-width="1.5"/>
            <rect x="92" y="50" width="16" height="10" fill={fill_of(fills, "neck")} stroke=outline stroke-width="1"/>

            // Torso (shoulders → waist → hips)
            <path d="M 56,68 Q 100,58 144,68 L 158,98 L 154,200 L 138,260 L 62,260 L 46,200 L 42,98 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>

            // Arms (upper)
            <path d="M 42,98 L 22,150 L 18,210 L 32,212 L 44,170 L 50,118 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>
            <path d="M 158,98 L 178,150 L 182,210 L 168,212 L 156,170 L 150,118 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>

            // Forearms (lower arms)
            <path d="M 18,210 L 14,280 L 26,290 L 36,220 Z"
                  fill={fill_of(fills, "forearms")} stroke=outline stroke-width="1"/>
            <path d="M 182,210 L 186,280 L 174,290 L 164,220 Z"
                  fill={fill_of(fills, "forearms")} stroke=outline stroke-width="1"/>

            // Legs
            <path d="M 62,260 L 86,260 L 96,360 L 86,430 L 60,432 L 56,360 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>
            <path d="M 138,260 L 114,260 L 104,360 L 114,430 L 140,432 L 144,360 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>

            // ── Muscle regions (overlays) ──────────────────────────────
            // Shoulders (front delts)
            <path d="M 46,72 Q 38,90 50,108 L 70,98 L 64,76 Z"
                  fill={fill_of(fills, "shoulders")} stroke=outline stroke-width="0.8"/>
            <path d="M 154,72 Q 162,90 150,108 L 130,98 L 136,76 Z"
                  fill={fill_of(fills, "shoulders")} stroke=outline stroke-width="0.8"/>

            // Chest (pectorals)
            <path d="M 64,78 Q 78,72 96,76 L 98,138 L 70,140 Q 56,128 60,108 Z"
                  fill={fill_of(fills, "chest")} stroke=outline stroke-width="0.8"/>
            <path d="M 136,78 Q 122,72 104,76 L 102,138 L 130,140 Q 144,128 140,108 Z"
                  fill={fill_of(fills, "chest")} stroke=outline stroke-width="0.8"/>

            // Biceps
            <path d="M 28,128 Q 22,156 32,200 L 46,196 Q 52,160 46,128 Z"
                  fill={fill_of(fills, "biceps")} stroke=outline stroke-width="0.8"/>
            <path d="M 172,128 Q 178,156 168,200 L 154,196 Q 148,160 154,128 Z"
                  fill={fill_of(fills, "biceps")} stroke=outline stroke-width="0.8"/>

            // Abdominals
            <path d="M 78,140 L 122,140 L 124,212 Q 100,220 76,212 Z"
                  fill={fill_of(fills, "abdominals")} stroke=outline stroke-width="0.8"/>

            // Quadriceps
            <path d="M 64,266 L 92,266 L 96,360 L 70,360 Z"
                  fill={fill_of(fills, "quadriceps")} stroke=outline stroke-width="0.8"/>
            <path d="M 136,266 L 108,266 L 104,360 L 130,360 Z"
                  fill={fill_of(fills, "quadriceps")} stroke=outline stroke-width="0.8"/>

            // Adductors (inner thigh)
            <path d="M 92,266 L 100,266 L 100,340 L 96,340 Z"
                  fill={fill_of(fills, "adductors")} stroke=outline stroke-width="0.8"/>
            <path d="M 108,266 L 100,266 L 100,340 L 104,340 Z"
                  fill={fill_of(fills, "adductors")} stroke=outline stroke-width="0.8"/>

            // Calves (visible from front as shin/calf sliver)
            <path d="M 70,372 L 88,372 L 86,424 L 66,424 Z"
                  fill={fill_of(fills, "calves")} stroke=outline stroke-width="0.8"/>
            <path d="M 130,372 L 112,372 L 114,424 L 134,424 Z"
                  fill={fill_of(fills, "calves")} stroke=outline stroke-width="0.8"/>

            // Label
            <text x="100" y="10" text-anchor="middle" font-size="10" fill="#a594cc" font-weight="600">"FRONT"</text>
        </svg>
    }
}

fn render_back(fills: &Fills) -> impl IntoView {
    let outline = "#3d2878";

    view! {
        <svg class="body-svg" viewBox="-30 -10 260 480" xmlns="http://www.w3.org/2000/svg">
            // Silhouette base (same outline as front)
            <circle cx="100" cy="30" r="22" fill="#2d2050" stroke=outline stroke-width="1.5"/>
            <rect x="92" y="50" width="16" height="10" fill={fill_of(fills, "neck")} stroke=outline stroke-width="1"/>

            <path d="M 56,68 Q 100,58 144,68 L 158,98 L 154,200 L 138,260 L 62,260 L 46,200 L 42,98 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>

            <path d="M 42,98 L 22,150 L 18,210 L 32,212 L 44,170 L 50,118 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>
            <path d="M 158,98 L 178,150 L 182,210 L 168,212 L 156,170 L 150,118 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>

            <path d="M 18,210 L 14,280 L 26,290 L 36,220 Z"
                  fill={fill_of(fills, "forearms")} stroke=outline stroke-width="1"/>
            <path d="M 182,210 L 186,280 L 174,290 L 164,220 Z"
                  fill={fill_of(fills, "forearms")} stroke=outline stroke-width="1"/>

            <path d="M 62,260 L 86,260 L 96,360 L 86,430 L 60,432 L 56,360 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>
            <path d="M 138,260 L 114,260 L 104,360 L 114,430 L 140,432 L 144,360 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>

            // ── Muscle regions (back) ──────────────────────────────────
            // Shoulders (rear delts)
            <path d="M 46,72 Q 38,90 50,108 L 70,98 L 64,76 Z"
                  fill={fill_of(fills, "shoulders")} stroke=outline stroke-width="0.8"/>
            <path d="M 154,72 Q 162,90 150,108 L 130,98 L 136,76 Z"
                  fill={fill_of(fills, "shoulders")} stroke=outline stroke-width="0.8"/>

            // Traps
            <path d="M 78,68 Q 100,62 122,68 L 116,108 L 100,114 L 84,108 Z"
                  fill={fill_of(fills, "traps")} stroke=outline stroke-width="0.8"/>

            // Middle back (upper)
            <path d="M 70,110 L 130,110 L 126,150 L 74,150 Z"
                  fill={fill_of(fills, "middle back")} stroke=outline stroke-width="0.8"/>

            // Lats
            <path d="M 60,116 Q 50,150 56,196 L 84,190 L 82,116 Z"
                  fill={fill_of(fills, "lats")} stroke=outline stroke-width="0.8"/>
            <path d="M 140,116 Q 150,150 144,196 L 116,190 L 118,116 Z"
                  fill={fill_of(fills, "lats")} stroke=outline stroke-width="0.8"/>

            // Lower back
            <path d="M 76,180 L 124,180 L 130,220 L 70,220 Z"
                  fill={fill_of(fills, "lower back")} stroke=outline stroke-width="0.8"/>

            // Triceps
            <path d="M 28,128 Q 22,156 32,200 L 46,196 Q 52,160 46,128 Z"
                  fill={fill_of(fills, "triceps")} stroke=outline stroke-width="0.8"/>
            <path d="M 172,128 Q 178,156 168,200 L 154,196 Q 148,160 154,128 Z"
                  fill={fill_of(fills, "triceps")} stroke=outline stroke-width="0.8"/>

            // Glutes
            <path d="M 62,228 Q 84,224 100,232 Q 116,224 138,228 L 134,272 Q 100,282 66,272 Z"
                  fill={fill_of(fills, "glutes")} stroke=outline stroke-width="0.8"/>

            // Abductors (outer hip — slivers at glute sides)
            <path d="M 62,228 L 60,260 L 56,260 L 56,232 Z"
                  fill={fill_of(fills, "abductors")} stroke=outline stroke-width="0.8"/>
            <path d="M 138,228 L 140,260 L 144,260 L 144,232 Z"
                  fill={fill_of(fills, "abductors")} stroke=outline stroke-width="0.8"/>

            // Hamstrings
            <path d="M 64,275 L 92,275 L 96,360 L 70,360 Z"
                  fill={fill_of(fills, "hamstrings")} stroke=outline stroke-width="0.8"/>
            <path d="M 136,275 L 108,275 L 104,360 L 130,360 Z"
                  fill={fill_of(fills, "hamstrings")} stroke=outline stroke-width="0.8"/>

            // Calves
            <path d="M 70,372 L 88,372 L 86,424 L 66,424 Z"
                  fill={fill_of(fills, "calves")} stroke=outline stroke-width="0.8"/>
            <path d="M 130,372 L 112,372 L 114,424 L 134,424 Z"
                  fill={fill_of(fills, "calves")} stroke=outline stroke-width="0.8"/>

            <text x="100" y="10" text-anchor="middle" font-size="10" fill="#a594cc" font-weight="600">"BACK"</text>
        </svg>
    }
}

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
            // Head
            <circle cx="100" cy="32" r="20" fill="#2d2050" stroke=outline stroke-width="1.5"/>
            // Neck
            <path d="M 88,50 C 90,58 90,62 86,68 L 114,68 C 110,62 110,58 112,50 Z"
                  fill={fill_of(fills, "neck")} stroke=outline stroke-width="1"/>

            // Torso (shoulders → ribs → waist → hips)
            <path d="M 86,68 C 70,68 56,76 50,90 C 44,108 44,128 46,150 C 48,176 50,196 56,214 C 62,234 64,250 60,266 C 70,270 86,272 100,272 C 114,272 130,270 140,266 C 136,250 138,234 144,214 C 150,196 152,176 154,150 C 156,128 156,108 150,90 C 144,76 130,68 114,68 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>

            // Upper arms
            <path d="M 50,90 C 38,108 30,138 26,170 C 24,188 26,202 30,212 C 38,210 44,200 46,186 C 50,162 52,138 54,118 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>
            <path d="M 150,90 C 162,108 170,138 174,170 C 176,188 174,202 170,212 C 162,210 156,200 154,186 C 150,162 148,138 146,118 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>

            // Forearms
            <path d="M 30,212 C 26,236 22,266 20,290 C 28,294 36,290 38,278 C 42,256 44,234 44,214 Z"
                  fill={fill_of(fills, "forearms")} stroke=outline stroke-width="1"/>
            <path d="M 170,212 C 174,236 178,266 180,290 C 172,294 164,290 162,278 C 158,256 156,234 156,214 Z"
                  fill={fill_of(fills, "forearms")} stroke=outline stroke-width="1"/>

            // Legs
            <path d="M 60,266 C 56,300 56,340 60,378 C 62,404 64,420 66,432 C 76,434 88,432 92,428 C 96,408 96,378 98,344 C 98,316 96,290 96,272 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>
            <path d="M 140,266 C 144,300 144,340 140,378 C 138,404 136,420 134,432 C 124,434 112,432 108,428 C 104,408 104,378 102,344 C 102,316 104,290 104,272 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>

            // ── Muscle regions (overlays) ──────────────────────────────
            // Shoulders (front delts)
            <path d="M 50,90 C 44,98 42,108 44,120 C 56,118 66,108 70,96 C 64,90 56,88 50,90 Z"
                  fill={fill_of(fills, "shoulders")} stroke=outline stroke-width="0.6"/>
            <path d="M 150,90 C 156,98 158,108 156,120 C 144,118 134,108 130,96 C 136,90 144,88 150,90 Z"
                  fill={fill_of(fills, "shoulders")} stroke=outline stroke-width="0.6"/>

            // Chest (pectorals)
            <path d="M 70,96 C 74,90 86,86 98,88 L 98,140 C 86,144 74,142 66,134 C 60,122 62,108 70,96 Z"
                  fill={fill_of(fills, "chest")} stroke=outline stroke-width="0.6"/>
            <path d="M 130,96 C 126,90 114,86 102,88 L 102,140 C 114,144 126,142 134,134 C 140,122 138,108 130,96 Z"
                  fill={fill_of(fills, "chest")} stroke=outline stroke-width="0.6"/>

            // Biceps
            <path d="M 34,128 C 28,150 30,180 38,206 C 46,202 50,184 50,164 C 50,148 46,134 42,124 Z"
                  fill={fill_of(fills, "biceps")} stroke=outline stroke-width="0.6"/>
            <path d="M 166,128 C 172,150 170,180 162,206 C 154,202 150,184 150,164 C 150,148 154,134 158,124 Z"
                  fill={fill_of(fills, "biceps")} stroke=outline stroke-width="0.6"/>

            // Abdominals
            <path d="M 86,144 C 92,142 108,142 114,144 L 114,212 C 108,218 92,218 86,212 Z"
                  fill={fill_of(fills, "abdominals")} stroke=outline stroke-width="0.6"/>

            // Quadriceps
            <path d="M 64,278 C 60,310 60,344 66,374 C 78,372 88,372 92,368 C 92,338 92,308 90,278 Z"
                  fill={fill_of(fills, "quadriceps")} stroke=outline stroke-width="0.6"/>
            <path d="M 136,278 C 140,310 140,344 134,374 C 122,372 112,372 108,368 C 108,338 108,308 110,278 Z"
                  fill={fill_of(fills, "quadriceps")} stroke=outline stroke-width="0.6"/>

            // Adductors (inner thigh slivers)
            <path d="M 92,278 L 100,278 L 100,344 L 96,344 Z"
                  fill={fill_of(fills, "adductors")} stroke=outline stroke-width="0.6"/>
            <path d="M 108,278 L 100,278 L 100,344 L 104,344 Z"
                  fill={fill_of(fills, "adductors")} stroke=outline stroke-width="0.6"/>

            // Calves (front sliver)
            <path d="M 70,386 C 70,406 72,420 76,432 C 84,432 90,428 92,418 C 90,402 88,392 86,384 Z"
                  fill={fill_of(fills, "calves")} stroke=outline stroke-width="0.6"/>
            <path d="M 130,386 C 130,406 128,420 124,432 C 116,432 110,428 108,418 C 110,402 112,392 114,384 Z"
                  fill={fill_of(fills, "calves")} stroke=outline stroke-width="0.6"/>

            // Label
            <text x="100" y="466" text-anchor="middle" font-size="9" fill="#a594cc" font-weight="700" letter-spacing="0.15em">"FRONT"</text>
        </svg>
    }
}

fn render_back(fills: &Fills) -> impl IntoView {
    let outline = "#3d2878";

    view! {
        <svg class="body-svg" viewBox="-30 -10 260 480" xmlns="http://www.w3.org/2000/svg">
            // Silhouette base (same outline as front)
            <circle cx="100" cy="32" r="20" fill="#2d2050" stroke=outline stroke-width="1.5"/>
            <path d="M 88,50 C 90,58 90,62 86,68 L 114,68 C 110,62 110,58 112,50 Z"
                  fill={fill_of(fills, "neck")} stroke=outline stroke-width="1"/>

            <path d="M 86,68 C 70,68 56,76 50,90 C 44,108 44,128 46,150 C 48,176 50,196 56,214 C 62,234 64,250 60,266 C 70,270 86,272 100,272 C 114,272 130,270 140,266 C 136,250 138,234 144,214 C 150,196 152,176 154,150 C 156,128 156,108 150,90 C 144,76 130,68 114,68 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>

            <path d="M 50,90 C 38,108 30,138 26,170 C 24,188 26,202 30,212 C 38,210 44,200 46,186 C 50,162 52,138 54,118 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>
            <path d="M 150,90 C 162,108 170,138 174,170 C 176,188 174,202 170,212 C 162,210 156,200 154,186 C 150,162 148,138 146,118 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>

            <path d="M 30,212 C 26,236 22,266 20,290 C 28,294 36,290 38,278 C 42,256 44,234 44,214 Z"
                  fill={fill_of(fills, "forearms")} stroke=outline stroke-width="1"/>
            <path d="M 170,212 C 174,236 178,266 180,290 C 172,294 164,290 162,278 C 158,256 156,234 156,214 Z"
                  fill={fill_of(fills, "forearms")} stroke=outline stroke-width="1"/>

            <path d="M 60,266 C 56,300 56,340 60,378 C 62,404 64,420 66,432 C 76,434 88,432 92,428 C 96,408 96,378 98,344 C 98,316 96,290 96,272 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>
            <path d="M 140,266 C 144,300 144,340 140,378 C 138,404 136,420 134,432 C 124,434 112,432 108,428 C 104,408 104,378 102,344 C 102,316 104,290 104,272 Z"
                  fill="#2d2050" stroke=outline stroke-width="1.5"/>

            // ── Muscle regions (back) ──────────────────────────────────
            // Traps
            <path d="M 78,68 C 88,62 112,62 122,68 C 118,84 110,98 100,114 C 90,98 82,84 78,68 Z"
                  fill={fill_of(fills, "traps")} stroke=outline stroke-width="0.6"/>

            // Shoulders (rear delts)
            <path d="M 50,90 C 44,100 42,114 46,124 C 58,122 68,112 70,98 Z"
                  fill={fill_of(fills, "shoulders")} stroke=outline stroke-width="0.6"/>
            <path d="M 150,90 C 156,100 158,114 154,124 C 142,122 132,112 130,98 Z"
                  fill={fill_of(fills, "shoulders")} stroke=outline stroke-width="0.6"/>

            // Middle back (upper)
            <path d="M 76,108 C 92,104 108,104 124,108 C 124,128 124,140 122,148 C 108,152 92,152 78,148 C 76,140 76,128 76,108 Z"
                  fill={fill_of(fills, "middle back")} stroke=outline stroke-width="0.6"/>

            // Lats
            <path d="M 62,116 C 52,138 50,170 58,200 C 70,196 80,188 84,176 C 86,160 86,138 84,118 Z"
                  fill={fill_of(fills, "lats")} stroke=outline stroke-width="0.6"/>
            <path d="M 138,116 C 148,138 150,170 142,200 C 130,196 120,188 116,176 C 114,160 114,138 116,118 Z"
                  fill={fill_of(fills, "lats")} stroke=outline stroke-width="0.6"/>

            // Lower back
            <path d="M 78,196 C 92,194 108,194 122,196 C 124,210 126,222 126,232 C 108,236 92,236 74,232 C 74,222 76,210 78,196 Z"
                  fill={fill_of(fills, "lower back")} stroke=outline stroke-width="0.6"/>

            // Triceps
            <path d="M 34,128 C 28,150 30,180 38,206 C 46,202 50,184 50,164 C 50,148 46,134 42,124 Z"
                  fill={fill_of(fills, "triceps")} stroke=outline stroke-width="0.6"/>
            <path d="M 166,128 C 172,150 170,180 162,206 C 154,202 150,184 150,164 C 150,148 154,134 158,124 Z"
                  fill={fill_of(fills, "triceps")} stroke=outline stroke-width="0.6"/>

            // Glutes
            <path d="M 62,236 C 76,232 90,234 100,242 C 110,234 124,232 138,236 C 138,256 130,270 116,276 C 104,278 96,278 84,276 C 70,270 62,256 62,236 Z"
                  fill={fill_of(fills, "glutes")} stroke=outline stroke-width="0.6"/>

            // Abductors (outer hip slivers)
            <path d="M 60,236 L 58,266 L 54,266 L 56,238 Z"
                  fill={fill_of(fills, "abductors")} stroke=outline stroke-width="0.6"/>
            <path d="M 140,236 L 142,266 L 146,266 L 144,238 Z"
                  fill={fill_of(fills, "abductors")} stroke=outline stroke-width="0.6"/>

            // Hamstrings
            <path d="M 64,278 C 60,310 60,344 66,374 C 78,372 88,372 92,368 C 92,338 92,308 90,278 Z"
                  fill={fill_of(fills, "hamstrings")} stroke=outline stroke-width="0.6"/>
            <path d="M 136,278 C 140,310 140,344 134,374 C 122,372 112,372 108,368 C 108,338 108,308 110,278 Z"
                  fill={fill_of(fills, "hamstrings")} stroke=outline stroke-width="0.6"/>

            // Calves
            <path d="M 70,386 C 70,406 72,420 76,432 C 84,432 90,428 92,418 C 90,402 88,392 86,384 Z"
                  fill={fill_of(fills, "calves")} stroke=outline stroke-width="0.6"/>
            <path d="M 130,386 C 130,406 128,420 124,432 C 116,432 110,428 108,418 C 110,402 112,392 114,384 Z"
                  fill={fill_of(fills, "calves")} stroke=outline stroke-width="0.6"/>

            <text x="100" y="466" text-anchor="middle" font-size="9" fill="#a594cc" font-weight="700" letter-spacing="0.15em">"BACK"</text>
        </svg>
    }
}

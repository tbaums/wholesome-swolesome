pub mod csv_utils;
pub mod models;
pub mod seed;
pub mod sync;

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    use crate::csv_utils::{export_plan_csv, import_plan_csv};
    use crate::models::{Exercise, ExerciseCategory, WorkoutDay, WorkoutPlan};

    fn simple_plan() -> WorkoutPlan {
        WorkoutPlan {
            days: vec![
                WorkoutDay {
                    id: "d1".into(),
                    name: "Push Day".into(),
                    exercises: vec![
                        Exercise {
                            id: "e1".into(),
                            name: "Bench Press".into(),
                            target_sets: 3,
                            reps_min: 8,
                            reps_max: 12,
                            category: ExerciseCategory::Main,
                            notes: None,
                        },
                        Exercise {
                            id: "e2".into(),
                            name: "Overhead Press".into(),
                            target_sets: 3,
                            reps_min: 6,
                            reps_max: 10,
                            category: ExerciseCategory::Main,
                            notes: Some("strict form".into()),
                        },
                    ],
                },
                WorkoutDay {
                    id: "d2".into(),
                    name: "Pull Day".into(),
                    exercises: vec![Exercise {
                        id: "e3".into(),
                        name: "Pull-up".into(),
                        target_sets: 4,
                        reps_min: 5,
                        reps_max: 10,
                        category: ExerciseCategory::Main,
                        notes: None,
                    }],
                },
            ],
        }
    }

    // Test 22: exported CSV has correct header and data rows
    #[wasm_bindgen_test]
    fn csv_export_plan_format() {
        let plan = simple_plan();
        let csv = export_plan_csv(&plan);
        let lines: Vec<&str> = csv.lines().collect();

        assert_eq!(
            lines[0],
            "day_id,day_name,exercise_id,exercise_name,target_sets,reps_min,reps_max,category,notes"
        );
        assert_eq!(lines[1], "d1,Push Day,e1,Bench Press,3,8,12,Main,");
        assert_eq!(lines[2], "d1,Push Day,e2,Overhead Press,3,6,10,Main,strict form");
        assert_eq!(lines[3], "d2,Pull Day,e3,Pull-up,4,5,10,Main,");
        assert_eq!(lines.len(), 4); // header + 3 exercises, no trailing blank line
    }

    // Test 23: import parses correctly and produces equivalent plan
    #[wasm_bindgen_test]
    fn csv_import_round_trip() {
        let original = simple_plan();
        let csv = export_plan_csv(&original);
        let imported = import_plan_csv(&csv).expect("import should succeed");

        assert_eq!(imported.days.len(), 2);
        assert_eq!(imported.days[0].id, "d1");
        assert_eq!(imported.days[0].name, "Push Day");
        assert_eq!(imported.days[0].exercises.len(), 2);
        assert_eq!(imported.days[0].exercises[0].name, "Bench Press");
        assert_eq!(imported.days[0].exercises[0].target_sets, 3);
        assert_eq!(imported.days[0].exercises[0].reps_min, 8);
        assert_eq!(imported.days[0].exercises[0].reps_max, 12);
        assert_eq!(imported.days[0].exercises[1].name, "Overhead Press");
        assert_eq!(
            imported.days[0].exercises[1].notes,
            Some("strict form".into())
        );
        assert_eq!(imported.days[1].exercises[0].name, "Pull-up");
        assert_eq!(imported.days[1].exercises[0].target_sets, 4);
        // category round-trips
        assert_eq!(
            imported.days[0].exercises[0].category,
            ExerciseCategory::Main
        );
    }

    // ── Sync module tests ─────────────────────────────────────────────────────

    use crate::sync::{SyncConfig, SyncedState};
    use base64::Engine;

    // Test: default SyncConfig is not configured (empty token + repo)
    #[wasm_bindgen_test]
    fn sync_config_unconfigured_when_empty() {
        let cfg = SyncConfig::default();
        assert!(!cfg.is_configured());
    }

    // Test: token present but no repo → not configured
    #[wasm_bindgen_test]
    fn sync_config_unconfigured_without_repo() {
        let cfg = SyncConfig { token: "tok".into(), ..SyncConfig::default() };
        assert!(!cfg.is_configured());
    }

    // Test: repo present but no token → not configured
    #[wasm_bindgen_test]
    fn sync_config_unconfigured_without_token() {
        let cfg = SyncConfig { repo: "owner/repo".into(), ..SyncConfig::default() };
        assert!(!cfg.is_configured());
    }

    // Test: to_github_config fills "main" / "state.json" when branch/path are empty
    #[wasm_bindgen_test]
    fn sync_config_fills_default_branch_and_path() {
        let cfg = SyncConfig {
            token: "tok".into(),
            repo: "owner/repo".into(),
            branch: String::new(),
            path: String::new(),
        };
        let gh = cfg.to_github_config();
        assert_eq!(gh.branch, "main");
        assert_eq!(gh.path, "state.json");
        assert_eq!(gh.repo, "owner/repo");
        assert_eq!(gh.token, "tok");
    }

    // Test: explicit branch/path are preserved
    #[wasm_bindgen_test]
    fn sync_config_preserves_explicit_branch_and_path() {
        let cfg = SyncConfig {
            token: "tok".into(),
            repo: "owner/repo".into(),
            branch: "dev".into(),
            path: "data/state.json".into(),
        };
        let gh = cfg.to_github_config();
        assert_eq!(gh.branch, "dev");
        assert_eq!(gh.path, "data/state.json");
    }

    // Test: SyncedState serializes and deserializes cleanly
    #[wasm_bindgen_test]
    fn synced_state_round_trip() {
        let state = SyncedState {
            schema_version: 1,
            updated_at: Some("2026-05-22T00:00:00.000Z".into()),
            plan: None,
            exercise_history: vec![],
            session_drafts: vec![],
            custom_exercises: vec![],
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: SyncedState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.schema_version, 1);
        assert_eq!(parsed.updated_at.as_deref(), Some("2026-05-22T00:00:00.000Z"));
        assert!(parsed.exercise_history.is_empty());
        assert!(parsed.plan.is_none());
    }

    // Test: partial JSON (missing array fields) deserializes via #[serde(default)]
    #[wasm_bindgen_test]
    fn synced_state_missing_arrays_default_to_empty() {
        let json = r#"{"schema_version":1,"updated_at":null}"#;
        let parsed: SyncedState = serde_json::from_str(json).unwrap();
        assert!(parsed.exercise_history.is_empty());
        assert!(parsed.session_drafts.is_empty());
        assert!(parsed.custom_exercises.is_empty());
        assert!(parsed.plan.is_none());
    }

    // Test: GitHub wraps base64 at 60 chars with newlines — our stripping logic handles it
    #[wasm_bindgen_test]
    fn github_base64_whitespace_strip_round_trips() {
        let original = r#"{"schema_version":1,"updated_at":null,"plan":null,"exercise_history":[],"session_drafts":[],"custom_exercises":[]}"#;
        let encoded = base64::engine::general_purpose::STANDARD.encode(original.as_bytes());
        // Simulate GitHub's 60-char line wrapping
        let wrapped: String = encoded
            .chars()
            .enumerate()
            .flat_map(|(i, c)| if i > 0 && i % 60 == 0 { vec!['\n', c] } else { vec![c] })
            .collect();
        let cleaned: String = wrapped.chars().filter(|c| !c.is_whitespace()).collect();
        let decoded = base64::engine::general_purpose::STANDARD.decode(&cleaned).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), original);
    }
}

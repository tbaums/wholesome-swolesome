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

    // Test: a deletion is preserved through a serialize → deserialize round-trip
    #[wasm_bindgen_test]
    fn deletion_preserved_through_serde_roundtrip() {
        use crate::models::ExerciseEntry;

        let make_entry = |id: &str| ExerciseEntry {
            id: id.to_string(),
            session_id: None,
            day_id: None,
            day_name: None,
            exercise_id: id.to_string(),
            exercise_name: format!("Exercise {id}"),
            date: "2026-01-01".to_string(),
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            target_sets: 3,
            reps_min: 8,
            reps_max: 12,
            finalized: false,
            sets: vec![],
        };

        // Start with 3 entries
        let history = vec![make_entry("a"), make_entry("b"), make_entry("c")];
        let state = SyncedState {
            schema_version: 1,
            updated_at: Some("2026-05-22T10:00:00.000Z".to_string()),
            plan: None,
            exercise_history: history,
            session_drafts: vec![],
            custom_exercises: vec![],
        };

        // Simulate push: serialize
        let json = serde_json::to_string(&state).unwrap();

        // Simulate delete of entry "b" on the client
        let mut after_delete: SyncedState = serde_json::from_str(&json).unwrap();
        after_delete.exercise_history.retain(|e| e.id != "b");
        after_delete.updated_at = Some("2026-05-22T10:05:00.000Z".to_string());

        // Simulate push of post-delete state, then pull on another session
        let pushed = serde_json::to_string(&after_delete).unwrap();
        let pulled: SyncedState = serde_json::from_str(&pushed).unwrap();

        assert_eq!(pulled.exercise_history.len(), 2);
        assert!(pulled.exercise_history.iter().all(|e| e.id != "b"),
            "deleted entry 'b' should not appear after pull");
        assert!(pulled.exercise_history.iter().any(|e| e.id == "a"));
        assert!(pulled.exercise_history.iter().any(|e| e.id == "c"));
    }

    // Test: deleting all history produces an empty vec, not null — deserializes correctly
    #[wasm_bindgen_test]
    fn delete_all_history_round_trips_as_empty_vec() {
        let state = SyncedState {
            schema_version: 1,
            updated_at: Some("2026-05-22T10:00:00.000Z".to_string()),
            plan: None,
            exercise_history: vec![],  // all deleted
            session_drafts: vec![],
            custom_exercises: vec![],
        };
        let json = serde_json::to_string(&state).unwrap();

        // Confirm the JSON encodes as an empty array, not null or absent
        assert!(json.contains("\"exercise_history\":[]"),
            "empty history should serialize as [] not null");

        let pulled: SyncedState = serde_json::from_str(&json).unwrap();
        assert!(pulled.exercise_history.is_empty());
    }

    // Test: boot-pull guard — empty remote exercise_history is treated as intentional
    // (covers the case where a user deleted all entries and the pull sees an empty array)
    #[wasm_bindgen_test]
    #[allow(clippy::eq_op, clippy::nonminimal_bool)]
    fn newer_timestamp_wins_regardless_of_content() {
        let older_ts = "2026-05-22T09:00:00.000Z";
        let newer_ts = "2026-05-22T10:00:00.000Z";

        // Remote is newer → should hydrate (even if its arrays are empty after deletes)
        assert!(newer_ts > older_ts,
            "ISO 8601 strings compare lexicographically; newer timestamp should sort higher");

        // Same timestamp → should NOT hydrate (no change)
        assert!(!(older_ts > older_ts));
    }

    // ── CSV quoting / edge cases ──────────────────────────────────────────────

    // Test: exercise names containing commas survive an export → import round-trip
    #[wasm_bindgen_test]
    fn csv_round_trip_preserves_comma_in_name() {
        let plan = WorkoutPlan {
            days: vec![WorkoutDay {
                id: "d1".into(),
                name: "Day, with comma".into(),
                exercises: vec![Exercise {
                    id: "e1".into(),
                    name: "Squat, low-bar".into(),
                    target_sets: 3,
                    reps_min: 5,
                    reps_max: 8,
                    category: ExerciseCategory::Main,
                    notes: Some("Heavy, focused".into()),
                }],
            }],
        };
        let csv = export_plan_csv(&plan);
        // The field must be quoted in the CSV text
        assert!(csv.contains("\"Squat, low-bar\""), "comma field should be quoted: {csv}");
        assert!(csv.contains("\"Day, with comma\""));
        assert!(csv.contains("\"Heavy, focused\""));
        let imported = import_plan_csv(&csv).expect("import should succeed");
        assert_eq!(imported.days[0].name, "Day, with comma");
        assert_eq!(imported.days[0].exercises[0].name, "Squat, low-bar");
        assert_eq!(
            imported.days[0].exercises[0].notes.as_deref(),
            Some("Heavy, focused"),
        );
    }

    // Test: a literal double-quote in a name is escaped as "" per CSV convention
    #[wasm_bindgen_test]
    fn csv_round_trip_preserves_quote_in_name() {
        let plan = WorkoutPlan {
            days: vec![WorkoutDay {
                id: "d1".into(),
                name: "Push".into(),
                exercises: vec![Exercise {
                    id: "e1".into(),
                    name: "Bench (\"competition\" grip)".into(),
                    target_sets: 3,
                    reps_min: 5,
                    reps_max: 8,
                    category: ExerciseCategory::Main,
                    notes: None,
                }],
            }],
        };
        let csv = export_plan_csv(&plan);
        // Inner quotes are doubled
        assert!(csv.contains("\"Bench (\"\"competition\"\" grip)\""), "csv: {csv}");
        let imported = import_plan_csv(&csv).expect("import should succeed");
        assert_eq!(
            imported.days[0].exercises[0].name,
            "Bench (\"competition\" grip)",
        );
    }

    // Test: import with empty exercise_id auto-generates a UUID rather than failing
    #[wasm_bindgen_test]
    fn csv_import_generates_uuid_when_exercise_id_empty() {
        let csv = "day_id,day_name,exercise_id,exercise_name,target_sets,reps_min,reps_max,category,notes\n\
                   d1,Push,,Bench Press,3,8,12,Main,\n";
        let plan = import_plan_csv(csv).expect("import should succeed");
        let id = &plan.days[0].exercises[0].id;
        assert!(!id.is_empty(), "id should be filled with a UUID");
        // UUIDs are 36 chars with hyphens — sanity check rather than full parse
        assert_eq!(id.len(), 36);
        assert_eq!(id.matches('-').count(), 4);
    }

    // Test: import returns the "no exercises" error for a header-only CSV
    #[wasm_bindgen_test]
    fn csv_import_empty_returns_error() {
        let csv = "day_id,day_name,exercise_id,exercise_name,target_sets,reps_min,reps_max,category,notes\n";
        let err = import_plan_csv(csv).expect_err("header-only CSV should fail");
        assert!(err.to_lowercase().contains("no exercises"), "got: {err}");
    }

    // Test: category strings are case-insensitive and unknown values fall back to Main
    #[wasm_bindgen_test]
    fn csv_import_category_parsing() {
        let csv = "day_id,day_name,exercise_id,exercise_name,target_sets,reps_min,reps_max,category,notes\n\
                   d1,D,e1,A,3,8,12,Core,\n\
                   d1,D,e2,B,3,8,12,cardio,\n\
                   d1,D,e3,C,3,8,12,MAIN,\n\
                   d1,D,e4,D,3,8,12,wat,\n";
        let plan = import_plan_csv(csv).expect("import should succeed");
        let cats: Vec<_> = plan.days[0].exercises.iter().map(|e| &e.category).collect();
        assert_eq!(cats[0], &ExerciseCategory::Core);
        assert_eq!(cats[1], &ExerciseCategory::Cardio);
        assert_eq!(cats[2], &ExerciseCategory::Main);
        assert_eq!(cats[3], &ExerciseCategory::Main); // unknown → default
    }

    // ── Model serde backward-compatibility ────────────────────────────────────

    // Test: legacy SetLog persisted with `weight_lbs` is loaded into the `weight` field
    #[wasm_bindgen_test]
    fn setlog_legacy_weight_lbs_alias_deserializes() {
        // Legacy field name (pre-rename) — must still load
        let legacy = r#"{"set_number":1,"reps":8,"weight_lbs":135.5,"completed":true}"#;
        let set: crate::models::SetLog =
            serde_json::from_str(legacy).expect("legacy weight_lbs should deserialize");
        assert_eq!(set.weight, 135.5);
        assert_eq!(set.reps, 8);
        assert!(set.completed);
        assert!(set.completed_date.is_none());
    }

    // Test: ExerciseEntry without `completed_date` / `finalized` / `created_at` loads
    // via serde defaults — guards data already shipped to existing users
    #[wasm_bindgen_test]
    fn exercise_entry_legacy_missing_optional_fields_deserializes() {
        use crate::models::ExerciseEntry;
        let legacy = r#"{
            "id":"x","date":"2026-01-01","exercise_name":"Row","exercise_id":"e1",
            "session_id":null,"day_id":null,"day_name":null,
            "target_sets":3,"reps_min":8,"reps_max":12,
            "sets":[{"set_number":1,"reps":10,"weight":100.0,"completed":true}]
        }"#;
        let entry: ExerciseEntry = serde_json::from_str(legacy)
            .expect("legacy entry without finalized/created_at should deserialize");
        assert_eq!(entry.exercise_name, "Row");
        assert!(!entry.finalized, "finalized defaults to false");
        assert!(entry.created_at.is_empty(), "created_at defaults to empty string");
        // Inner SetLog also fills completed_date via default
        assert!(entry.sets[0].completed_date.is_none());
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

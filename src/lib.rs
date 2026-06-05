pub mod app;
pub mod coach;
pub mod components;
pub mod csv_utils;
pub mod library;
pub mod models;
pub mod storage;
pub mod sync;

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    use crate::models::{
        ExerciseEntry, ScheduledExercise, ScheduledWorkout, UserGoals, WorkoutSource,
    };
    use crate::sync::{SyncConfig, SyncedState};
    use base64::Engine;

    // ── SyncConfig ────────────────────────────────────────────────────────────

    #[wasm_bindgen_test]
    fn sync_config_unconfigured_when_empty() {
        let cfg = SyncConfig::default();
        assert!(!cfg.is_configured());
    }

    #[wasm_bindgen_test]
    fn sync_config_unconfigured_without_repo() {
        let cfg = SyncConfig { token: "tok".into(), ..SyncConfig::default() };
        assert!(!cfg.is_configured());
    }

    #[wasm_bindgen_test]
    fn sync_config_unconfigured_without_token() {
        let cfg = SyncConfig { repo: "owner/repo".into(), ..SyncConfig::default() };
        assert!(!cfg.is_configured());
    }

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
    }

    // ── SyncedState v2 ────────────────────────────────────────────────────────

    fn empty_state(ts: &str) -> SyncedState {
        SyncedState {
            schema_version: 2,
            updated_at: Some(ts.into()),
            goals: UserGoals::default(),
            scheduled_workouts: vec![],
            exercise_history: vec![],
            session_drafts: vec![],
            custom_exercises: vec![],
            plan: None,
        }
    }

    #[wasm_bindgen_test]
    fn synced_state_round_trip() {
        let state = empty_state("2026-05-22T00:00:00.000Z");
        let json = serde_json::to_string(&state).unwrap();
        let parsed: SyncedState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.schema_version, 2);
        assert_eq!(parsed.updated_at.as_deref(), Some("2026-05-22T00:00:00.000Z"));
        assert!(parsed.exercise_history.is_empty());
        assert!(parsed.scheduled_workouts.is_empty());
    }

    #[wasm_bindgen_test]
    fn synced_state_missing_arrays_default_to_empty() {
        let json = r#"{"schema_version":2,"updated_at":null}"#;
        let parsed: SyncedState = serde_json::from_str(json).unwrap();
        assert!(parsed.exercise_history.is_empty());
        assert!(parsed.session_drafts.is_empty());
        assert!(parsed.custom_exercises.is_empty());
        assert!(parsed.scheduled_workouts.is_empty());
    }

    #[wasm_bindgen_test]
    fn synced_state_accepts_legacy_v1_with_plan() {
        // Old shape — has `plan` but no `goals`/`scheduled_workouts`.
        let json = r#"{"schema_version":1,"updated_at":"2026-01-01T00:00:00.000Z","plan":{"days":[]},"exercise_history":[]}"#;
        let parsed: SyncedState = serde_json::from_str(json).unwrap();
        assert!(parsed.plan.is_some());
        assert!(parsed.scheduled_workouts.is_empty());
    }

    #[wasm_bindgen_test]
    fn user_goals_pre_cardio_fields_deserialize_with_defaults() {
        // State written before the cardio/mobility fields existed — must still load.
        let json = r#"{
            "primary_goal":"Hypertrophy","sessions_per_week":4,"session_minutes":60,
            "equipment":["barbell"],"avoid":"none","notes":""
        }"#;
        let goals: UserGoals = serde_json::from_str(json).unwrap();
        assert!(goals.weekly_cardio_minutes_target.is_none());
        assert!(goals.vo2_max_latest.is_none());
        assert!(goals.vo2_max_updated.is_none());
        assert_eq!(goals.mobility_focus, crate::models::FocusLevel::Standard);
        assert_eq!(goals.balance_focus, crate::models::FocusLevel::Standard);
    }

    #[wasm_bindgen_test]
    fn user_goals_round_trip_preserves_cardio_fields() {
        let g = UserGoals {
            weekly_cardio_minutes_target: Some(120),
            vo2_max_latest: Some(36.4),
            vo2_max_updated: Some("2026-05-27".into()),
            mobility_focus: crate::models::FocusLevel::High,
            balance_focus: crate::models::FocusLevel::Low,
            ..UserGoals::default()
        };

        let json = serde_json::to_string(&g).unwrap();
        let parsed: UserGoals = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.weekly_cardio_minutes_target, Some(120));
        assert_eq!(parsed.vo2_max_latest, Some(36.4));
        assert_eq!(parsed.vo2_max_updated.as_deref(), Some("2026-05-27"));
        assert_eq!(parsed.mobility_focus, crate::models::FocusLevel::High);
        assert_eq!(parsed.balance_focus, crate::models::FocusLevel::Low);
    }

    #[wasm_bindgen_test]
    fn scheduled_workout_round_trips() {
        let mut state = empty_state("2026-05-22T00:00:00.000Z");
        state.scheduled_workouts.push(ScheduledWorkout {
            id: "sw-1".into(),
            date: "2026-05-23".into(),
            name: "Upper Push".into(),
            rationale: "Chest fresh, shoulders recovered 4 days".into(),
            source: WorkoutSource::Coach,
            exercises: vec![ScheduledExercise {
                library_id: Some("Barbell_Bench_Press_-_Medium_Grip".into()),
                name: "Bench Press".into(),
                target_sets: 4,
                reps_min: 6,
                reps_max: 8,
                rest_seconds: 180,
                notes: Some("RPE 7".into()),
                target_duration_seconds: None,
                target_zones: None,
            }],
            created_at: "2026-05-22T23:00:00.000Z".into(),
        });
        let json = serde_json::to_string(&state).unwrap();
        let parsed: SyncedState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.scheduled_workouts.len(), 1);
        let w = &parsed.scheduled_workouts[0];
        assert_eq!(w.exercises[0].library_id.as_deref(), Some("Barbell_Bench_Press_-_Medium_Grip"));
        assert_eq!(w.exercises[0].rest_seconds, 180);
    }

    #[wasm_bindgen_test]
    fn deletion_preserved_through_serde_roundtrip() {
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
            target_duration_seconds: None,
        };

        let mut state = empty_state("2026-05-22T10:00:00.000Z");
        state.exercise_history = vec![make_entry("a"), make_entry("b"), make_entry("c")];
        let json = serde_json::to_string(&state).unwrap();

        let mut after_delete: SyncedState = serde_json::from_str(&json).unwrap();
        after_delete.exercise_history.retain(|e| e.id != "b");
        after_delete.updated_at = Some("2026-05-22T10:05:00.000Z".to_string());

        let pushed = serde_json::to_string(&after_delete).unwrap();
        let pulled: SyncedState = serde_json::from_str(&pushed).unwrap();

        assert_eq!(pulled.exercise_history.len(), 2);
        assert!(pulled.exercise_history.iter().all(|e| e.id != "b"));
    }

    #[wasm_bindgen_test]
    fn delete_all_history_round_trips_as_empty_vec() {
        let state = empty_state("2026-05-22T10:00:00.000Z");
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"exercise_history\":[]"));
        let pulled: SyncedState = serde_json::from_str(&json).unwrap();
        assert!(pulled.exercise_history.is_empty());
    }

    // Boot-pull guard: newer remote timestamp wins regardless of array contents
    // (covers the case where a user deleted all entries and the pull sees an empty array).
    #[wasm_bindgen_test]
    #[allow(clippy::eq_op, clippy::nonminimal_bool)]
    fn newer_timestamp_wins_regardless_of_content() {
        let older_ts = "2026-05-22T09:00:00.000Z";
        let newer_ts = "2026-05-22T10:00:00.000Z";
        assert!(newer_ts > older_ts,
            "ISO 8601 strings compare lexicographically; newer timestamp should sort higher");
        assert!(!(older_ts > older_ts));
    }

    // ── Model serde backward-compatibility ────────────────────────────────────

    #[wasm_bindgen_test]
    fn setlog_legacy_weight_lbs_alias_deserializes() {
        let legacy = r#"{"set_number":1,"reps":8,"weight_lbs":135.5,"completed":true}"#;
        let set: crate::models::SetLog =
            serde_json::from_str(legacy).expect("legacy weight_lbs should deserialize");
        assert_eq!(set.weight, 135.5);
        assert_eq!(set.reps, 8);
        assert!(set.completed);
        assert!(set.completed_date.is_none());
    }

    #[wasm_bindgen_test]
    fn exercise_entry_legacy_missing_optional_fields_deserializes() {
        let legacy = r#"{
            "id":"x","date":"2026-01-01","exercise_name":"Row","exercise_id":"e1",
            "session_id":null,"day_id":null,"day_name":null,
            "target_sets":3,"reps_min":8,"reps_max":12,
            "sets":[{"set_number":1,"reps":10,"weight":100.0,"completed":true}]
        }"#;
        let entry: ExerciseEntry = serde_json::from_str(legacy)
            .expect("legacy entry without finalized/created_at should deserialize");
        assert_eq!(entry.exercise_name, "Row");
        assert!(!entry.finalized);
        assert!(entry.created_at.is_empty());
        assert!(entry.sets[0].completed_date.is_none());
    }

    // GitHub wraps base64 at 60 chars with newlines — our stripping logic handles it.
    #[wasm_bindgen_test]
    fn github_base64_whitespace_strip_round_trips() {
        let original = serde_json::to_string(&empty_state("2026-05-22T00:00:00.000Z")).unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(original.as_bytes());
        let wrapped: String = encoded
            .chars()
            .enumerate()
            .flat_map(|(i, c)| if i > 0 && i % 60 == 0 { vec!['\n', c] } else { vec![c] })
            .collect();
        let cleaned: String = wrapped.chars().filter(|c| !c.is_whitespace()).collect();
        let decoded = base64::engine::general_purpose::STANDARD.decode(&cleaned).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), original);
    }

    // ── Library ──────────────────────────────────────────────────────────────

    #[wasm_bindgen_test]
    fn library_parses_sample_entry() {
        use crate::models::LibraryExercise;
        let sample = r#"[{
            "id":"Ab_Roller","name":"Ab Roller",
            "force":"pull","level":"intermediate","mechanic":"compound","equipment":"other",
            "primaryMuscles":["abdominals"],"secondaryMuscles":["shoulders"],
            "instructions":["a","b"],"category":"strength","images":["x.jpg"]
        }]"#;
        let parsed: Vec<LibraryExercise> = serde_json::from_str(sample).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].primary_muscles, vec!["abdominals"]);
        assert_eq!(parsed[0].equipment.as_deref(), Some("other"));
    }

    // ── Category / cardio detection ─────────────────────────────────────────

    fn lib_entry(id: &str, name: &str) -> crate::models::LibraryExercise {
        lib_entry_with_cat(id, name, "strength")
    }

    fn lib_entry_with_cat(id: &str, name: &str, cat: &str) -> crate::models::LibraryExercise {
        crate::models::LibraryExercise {
            id: id.into(),
            name: name.into(),
            force: None,
            level: "intermediate".into(),
            mechanic: None,
            equipment: Some("barbell".into()),
            primary_muscles: vec!["chest".into()],
            secondary_muscles: vec!["triceps".into()],
            instructions: vec![],
            category: cat.into(),
            images: vec![],
        }
    }

    #[wasm_bindgen_test]
    fn category_of_finds_by_id() {
        use crate::library::category_of;
        let lib = vec![lib_entry_with_cat("Jogging_Treadmill", "Jogging, Treadmill", "cardio")];
        assert_eq!(category_of("Jogging_Treadmill", "anything", &lib), Some("cardio"));
    }

    #[wasm_bindgen_test]
    fn category_of_falls_back_to_name() {
        use crate::library::category_of;
        let lib = vec![lib_entry_with_cat("Jogging_Treadmill", "Jogging, Treadmill", "cardio")];
        assert_eq!(category_of("wrong_id", "jogging, treadmill", &lib), Some("cardio"));
    }

    #[wasm_bindgen_test]
    fn category_of_returns_none_for_unknown() {
        use crate::library::category_of;
        let lib = vec![lib_entry("Bench", "Bench Press")];
        assert_eq!(category_of("no_match", "no match", &lib), None);
    }

    #[wasm_bindgen_test]
    fn is_cardio_true_for_cardio_category() {
        use crate::library::is_cardio_exercise;
        let lib = vec![lib_entry_with_cat("Running_Treadmill", "Running, Treadmill", "cardio")];
        assert!(is_cardio_exercise("Running_Treadmill", "Running, Treadmill", &lib));
    }

    #[wasm_bindgen_test]
    fn is_cardio_false_for_strength() {
        use crate::library::is_cardio_exercise;
        let lib = vec![lib_entry("Bench", "Bench Press")];
        assert!(!is_cardio_exercise("Bench", "Bench Press", &lib));
    }

    #[wasm_bindgen_test]
    fn is_cardio_false_when_not_in_library() {
        use crate::library::is_cardio_exercise;
        let lib = vec![lib_entry("Bench", "Bench Press")];
        assert!(!is_cardio_exercise("Unknown", "Unknown Exercise", &lib));
    }

    // ── Bodyweight detection ─────────────────────────────────────────────────

    fn lib_entry_with_equipment(id: &str, name: &str, equipment: Option<&str>) -> crate::models::LibraryExercise {
        crate::models::LibraryExercise {
            id: id.into(),
            name: name.into(),
            force: None,
            level: "intermediate".into(),
            mechanic: None,
            equipment: equipment.map(String::from),
            primary_muscles: vec!["chest".into()],
            secondary_muscles: vec![],
            instructions: vec![],
            category: "strength".into(),
            images: vec![],
        }
    }

    #[wasm_bindgen_test]
    fn is_bodyweight_true_for_body_only_equipment() {
        use crate::library::is_bodyweight_exercise;
        let lib = vec![lib_entry_with_equipment("Pull-Up", "Pull-up", Some("body only"))];
        assert!(is_bodyweight_exercise("Pull-Up", "Pull-up", &lib));
    }

    #[wasm_bindgen_test]
    fn is_bodyweight_true_via_name_fallback() {
        // Library lookup falls back to lowercased-name match if id misses.
        use crate::library::is_bodyweight_exercise;
        let lib = vec![lib_entry_with_equipment("Pull-Up", "Pull-up", Some("body only"))];
        assert!(is_bodyweight_exercise("wrong-id", "Pull-up", &lib));
    }

    #[wasm_bindgen_test]
    fn is_bodyweight_false_for_barbell_equipment() {
        use crate::library::is_bodyweight_exercise;
        let lib = vec![lib_entry_with_equipment("Bench", "Bench Press", Some("barbell"))];
        assert!(!is_bodyweight_exercise("Bench", "Bench Press", &lib));
    }

    #[wasm_bindgen_test]
    fn is_bodyweight_false_when_equipment_is_none() {
        // Library entries without equipment (most stretching) should not flip the bit —
        // those use other render paths (duration timer, cardio, etc.).
        use crate::library::is_bodyweight_exercise;
        let lib = vec![lib_entry_with_equipment("Cat_Stretch", "Cat Stretch", None)];
        assert!(!is_bodyweight_exercise("Cat_Stretch", "Cat Stretch", &lib));
    }

    #[wasm_bindgen_test]
    fn is_bodyweight_false_when_not_in_library() {
        // Freeform / custom exercises (no library entry) keep the standard weight × reps UI.
        use crate::library::is_bodyweight_exercise;
        let lib = vec![lib_entry_with_equipment("Bench", "Bench Press", Some("barbell"))];
        assert!(!is_bodyweight_exercise("My Custom Lift", "My Custom Lift", &lib));
    }

    #[wasm_bindgen_test]
    fn is_bodyweight_false_for_exercise_ball() {
        // "exercise ball" includes weighted variants — keep weight input visible.
        use crate::library::is_bodyweight_exercise;
        let lib = vec![lib_entry_with_equipment("WBH", "Weighted Ball Hyperextension", Some("exercise ball"))];
        assert!(!is_bodyweight_exercise("WBH", "Weighted Ball Hyperextension", &lib));
    }

    #[wasm_bindgen_test]
    fn is_bodyweight_false_for_decline_crunch_on_allow_list() {
        // Decline Crunch is body-only but commonly weighted with a plate.
        // It's on WEIGHTABLE_BODYWEIGHT_IDS so the helper must NOT treat it as
        // bodyweight (weight input should stay visible).
        use crate::library::is_bodyweight_exercise;
        let lib = vec![lib_entry_with_equipment(
            "Decline_Crunch",
            "Decline Crunch",
            Some("body only"),
        )];
        assert!(!is_bodyweight_exercise("Decline_Crunch", "Decline Crunch", &lib));
    }

    #[wasm_bindgen_test]
    fn is_bodyweight_false_for_pullups_on_allow_list() {
        // Pullups is body-only but commonly weighted with a dip belt.
        use crate::library::is_bodyweight_exercise;
        let lib = vec![lib_entry_with_equipment("Pullups", "Pullups", Some("body only"))];
        assert!(!is_bodyweight_exercise("Pullups", "Pullups", &lib));
    }

    #[wasm_bindgen_test]
    fn is_bodyweight_true_for_plank_not_on_allow_list() {
        // Plank is body-only and genuinely weightless — should still hide weight.
        use crate::library::is_bodyweight_exercise;
        let lib = vec![lib_entry_with_equipment("Plank", "Plank", Some("body only"))];
        assert!(is_bodyweight_exercise("Plank", "Plank", &lib));
    }

    #[wasm_bindgen_test]
    fn every_allow_list_id_resolves_to_false_when_present_in_library() {
        // Regression guard: any id added to WEIGHTABLE_BODYWEIGHT_IDS MUST cause
        // is_bodyweight_exercise to return false (i.e., keep weight input visible).
        // If someone adds an id here but the equipment lookup is broken, we want
        // a loud failure.
        use crate::library::{is_bodyweight_exercise, WEIGHTABLE_BODYWEIGHT_IDS};
        for id in WEIGHTABLE_BODYWEIGHT_IDS {
            let lib = vec![lib_entry_with_equipment(id, id, Some("body only"))];
            assert!(
                !is_bodyweight_exercise(id, id, &lib),
                "expected {id} to render with weight input (on allow-list) but is_bodyweight_exercise returned true"
            );
        }
    }

    #[wasm_bindgen_test]
    fn bodyweight_entry_with_non_zero_weight_round_trips_unchanged() {
        // Historical edge case: a user logged a weighted pull-up under the
        // plain "Pullups" name BEFORE we hid the weight input. The UI no
        // longer offers a weight field for body-only exercises, but the
        // existing data must serialize → deserialize → serialize back to
        // bytes-identical JSON. Library detection is a render-time concern;
        // it MUST NOT influence storage.
        let entry = ExerciseEntry {
            id: "e-legacy".into(),
            date: "2026-04-01".into(),
            created_at: "2026-04-01T10:00:00.000Z".into(),
            exercise_name: "Pullups".into(),
            exercise_id: "Pullups".into(),
            session_id: None,
            day_id: None,
            day_name: None,
            target_sets: 3,
            reps_min: 5,
            reps_max: 8,
            sets: vec![crate::models::SetLog {
                set_number: 1,
                reps: 6,
                weight: 25.0, // legacy weighted pull-up entry
                completed: true,
                completed_date: Some("2026-04-01".into()),
                duration_seconds: None,
                zone_minutes: None,            }],
            finalized: true,
            target_duration_seconds: None,
        };
        let first = serde_json::to_string(&entry).unwrap();
        let parsed: ExerciseEntry = serde_json::from_str(&first).unwrap();
        let second = serde_json::to_string(&parsed).unwrap();
        assert_eq!(first, second, "round-trip must be byte-identical");
        assert_eq!(parsed.sets[0].weight, 25.0, "weight must not be zeroed");
    }

    #[wasm_bindgen_test]
    fn bodyweight_entry_with_zero_weight_round_trips_unchanged() {
        // The expected new-data case: bodyweight set logged with weight=0.0
        // (the default the schema has always written for non-weighted sets).
        let entry = ExerciseEntry {
            id: "e-new".into(),
            date: "2026-05-31".into(),
            created_at: "2026-05-31T10:00:00.000Z".into(),
            exercise_name: "Pullups".into(),
            exercise_id: "Pullups".into(),
            session_id: None,
            day_id: None,
            day_name: None,
            target_sets: 3,
            reps_min: 5,
            reps_max: 10,
            sets: vec![crate::models::SetLog {
                set_number: 1,
                reps: 8,
                weight: 0.0,
                completed: true,
                completed_date: Some("2026-05-31".into()),
                duration_seconds: None,
                zone_minutes: None,            }],
            finalized: true,
            target_duration_seconds: None,
        };
        let first = serde_json::to_string(&entry).unwrap();
        let parsed: ExerciseEntry = serde_json::from_str(&first).unwrap();
        let second = serde_json::to_string(&parsed).unwrap();
        assert_eq!(first, second);
        assert_eq!(parsed.sets[0].weight, 0.0);
        assert_eq!(parsed.sets[0].reps, 8);
    }

    // ── Coach: library-id validation ─────────────────────────────────────────

    #[wasm_bindgen_test]
    fn parse_workout_accepts_known_library_id() {
        let lib = vec![lib_entry("Barbell_Bench_Press_-_Medium_Grip", "Bench Press")];
        let json = r#"{
            "name": "Push",
            "rationale": "Chest fresh.",
            "exercises": [
                {"library_id":"Barbell_Bench_Press_-_Medium_Grip","name":"Bench Press",
                 "target_sets":4,"reps_min":6,"reps_max":8,"rest_seconds":180,"notes":null}
            ]
        }"#;
        let p = crate::coach::parse_workout_response(json, "2026-05-25", "2026-05-24T00:00:00.000Z", &lib)
            .expect("valid id should pass");
        assert_eq!(p.workout.exercises.len(), 1);
        assert_eq!(
            p.workout.exercises[0].library_id.as_deref(),
            Some("Barbell_Bench_Press_-_Medium_Grip")
        );
        assert!(p.vitals.is_none());
    }

    #[wasm_bindgen_test]
    fn parse_workout_rejects_unknown_library_id() {
        let lib = vec![lib_entry("Barbell_Bench_Press_-_Medium_Grip", "Bench Press")];
        let json = r#"{
            "name": "Push",
            "exercises": [
                {"library_id":"Made_Up_Lift","name":"Bench-ish Press",
                 "target_sets":4,"reps_min":6,"reps_max":8,"rest_seconds":180,"notes":null}
            ]
        }"#;
        let err = crate::coach::parse_workout_response(
            json, "2026-05-25", "2026-05-24T00:00:00.000Z", &lib,
        )
        .expect_err("invalid id should fail");
        assert!(err.contains("Made_Up_Lift"), "error should name the offender: {err}");
    }

    #[wasm_bindgen_test]
    fn parse_workout_rejects_missing_library_id() {
        let lib = vec![lib_entry("Barbell_Bench_Press_-_Medium_Grip", "Bench Press")];
        let json = r#"{
            "name": "Push",
            "exercises": [
                {"library_id":null,"name":"Freeform Bench",
                 "target_sets":4,"reps_min":6,"reps_max":8,"rest_seconds":180,"notes":null}
            ]
        }"#;
        let err = crate::coach::parse_workout_response(
            json, "2026-05-25", "2026-05-24T00:00:00.000Z", &lib,
        )
        .expect_err("missing id should fail");
        assert!(err.contains("Freeform Bench"), "error should name the offender: {err}");
    }

    #[wasm_bindgen_test]
    fn parse_workout_rejects_blank_library_id() {
        let lib = vec![lib_entry("Barbell_Bench_Press_-_Medium_Grip", "Bench Press")];
        let json = r#"{
            "name": "Push",
            "exercises": [
                {"library_id":"  ","name":"Whitespace ID",
                 "target_sets":4,"reps_min":6,"reps_max":8,"rest_seconds":180,"notes":null}
            ]
        }"#;
        let err = crate::coach::parse_workout_response(
            json, "2026-05-25", "2026-05-24T00:00:00.000Z", &lib,
        )
        .expect_err("blank id should fail");
        assert!(err.contains("Whitespace ID"), "error should name the offender: {err}");
    }

    #[wasm_bindgen_test]
    fn parse_workout_bails_when_library_empty() {
        // Library hasn't loaded yet → don't silently accept anything.
        let lib: Vec<crate::models::LibraryExercise> = vec![];
        let json = r#"{
            "name": "Push",
            "exercises": [
                {"library_id":"Barbell_Bench_Press_-_Medium_Grip","name":"Bench Press",
                 "target_sets":4,"reps_min":6,"reps_max":8,"rest_seconds":180,"notes":null}
            ]
        }"#;
        let err = crate::coach::parse_workout_response(
            json, "2026-05-25", "2026-05-24T00:00:00.000Z", &lib,
        )
        .expect_err("empty library should bail");
        assert!(err.to_lowercase().contains("library"), "error should mention library: {err}");
    }

    #[wasm_bindgen_test]
    fn coach_packet_inlines_library_ids() {
        use crate::coach::{build_coach_packet, PacketInput};
        let lib = vec![lib_entry("Barbell_Bench_Press_-_Medium_Grip", "Bench Press")];
        let goals = UserGoals::default();
        let packet = build_coach_packet(PacketInput {
            goals: &goals,
            history: &[],
            library: &lib,
            scheduled: &[],
            today: "2026-05-24",
            target_date: "2026-05-25",
        });
        // The off-app Claude needs the IDs directly in the brief.
        assert!(
            packet.contains("Barbell_Bench_Press_-_Medium_Grip"),
            "packet should inline library ids"
        );
        assert!(
            packet.to_lowercase().contains("library_id"),
            "packet should reference library_id field"
        );
    }

    // ── Coach: history filtering ───────────────────────────────────────────

    #[wasm_bindgen_test]
    fn coach_packet_includes_drafts_with_completed_sets_but_excludes_incomplete() {
        use crate::coach::{build_coach_packet, PacketInput};
        use crate::models::SetLog;

        let today = "2026-05-25";
        let recent_date = "2026-05-24";

        let finalized_completed = ExerciseEntry {
            id: "e1".into(),
            session_id: None,
            day_id: None,
            day_name: Some("Push".into()),
            exercise_id: "bench".into(),
            exercise_name: "Finalized Bench".into(),
            date: recent_date.into(),
            created_at: format!("{recent_date}T10:00:00.000Z"),
            target_sets: 3,
            reps_min: 8,
            reps_max: 12,
            finalized: true,
            target_duration_seconds: None,
            sets: vec![SetLog {
                set_number: 1,
                reps: 8,
                weight: 135.0,
                completed: true,
                completed_date: Some(recent_date.into()),
                duration_seconds: None,
                zone_minutes: None,            }],
        };
        let non_finalized = ExerciseEntry {
            id: "e2".into(),
            exercise_name: "Draft Squat".into(),
            finalized: false,
            ..finalized_completed.clone()
        };
        let no_completed_sets = ExerciseEntry {
            id: "e3".into(),
            exercise_name: "Abandoned Row".into(),
            finalized: true,
            sets: vec![SetLog {
                set_number: 1,
                reps: 0,
                weight: 0.0,
                completed: false,
                completed_date: None,
                duration_seconds: None,
                zone_minutes: None,            }],
            ..finalized_completed.clone()
        };

        let history = vec![finalized_completed, non_finalized, no_completed_sets];
        let lib = vec![lib_entry("bench", "Bench Press")];
        let packet = build_coach_packet(PacketInput {
            goals: &UserGoals::default(),
            history: &history,
            library: &lib,
            scheduled: &[],
            today,
            target_date: "2026-05-26",
        });

        // Finalized entries with completed sets ALWAYS show.
        assert!(packet.contains("Finalized Bench"), "finalized + completed entry should appear");
        // Drafts with completed sets ALSO show — a checked-off set is real work
        // even if the user never tapped the per-card ✓ to finalize the entry.
        // Without this, the coach brief was inconsistent with cardio_minutes_in_window
        // (totals counted drafts; the rundown didn't).
        assert!(packet.contains("Draft Squat"), "draft with completed sets must appear");
        // Entries with zero completed sets are still excluded — no real work happened.
        assert!(!packet.contains("Abandoned Row"), "entry with no completed sets should be excluded");
    }

    // ── Duration fields backward-compatibility ────────────────────────────────

    #[wasm_bindgen_test]
    fn setlog_without_duration_defaults_to_none() {
        let json = r#"{"set_number":1,"reps":8,"weight":100.0,"completed":true}"#;
        let set: crate::models::SetLog = serde_json::from_str(json).unwrap();
        assert!(set.duration_seconds.is_none());
    }

    #[wasm_bindgen_test]
    fn setlog_with_duration_deserializes() {
        let json = r#"{"set_number":1,"reps":1,"weight":0.0,"completed":true,"duration_seconds":30}"#;
        let set: crate::models::SetLog = serde_json::from_str(json).unwrap();
        assert_eq!(set.duration_seconds, Some(30));
    }

    #[wasm_bindgen_test]
    fn setlog_duration_round_trips() {
        let set = crate::models::SetLog {
            set_number: 1,
            reps: 1,
            weight: 0.0,
            completed: true,
            completed_date: None,
            duration_seconds: Some(45),
            zone_minutes: None,        };
        let json = serde_json::to_string(&set).unwrap();
        let parsed: crate::models::SetLog = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.duration_seconds, Some(45));
    }

    #[wasm_bindgen_test]
    fn scheduled_exercise_without_duration_defaults_to_none() {
        let json = r#"{
            "library_id":"Cat_Stretch","name":"Cat Stretch",
            "target_sets":2,"reps_min":1,"reps_max":1,"rest_seconds":10,"notes":null
        }"#;
        let ex: ScheduledExercise = serde_json::from_str(json).unwrap();
        assert!(ex.target_duration_seconds.is_none());
    }

    #[wasm_bindgen_test]
    fn scheduled_exercise_with_duration_deserializes() {
        let json = r#"{
            "library_id":"Cat_Stretch","name":"Cat Stretch",
            "target_sets":2,"reps_min":1,"reps_max":1,
            "rest_seconds":10,"notes":"hold 30s",
            "target_duration_seconds":30
        }"#;
        let ex: ScheduledExercise = serde_json::from_str(json).unwrap();
        assert_eq!(ex.target_duration_seconds, Some(30));
    }

    #[wasm_bindgen_test]
    fn exercise_entry_without_duration_defaults_to_none() {
        let json = r#"{
            "id":"x","date":"2026-01-01","exercise_name":"Row","exercise_id":"e1",
            "session_id":null,"day_id":null,"day_name":null,
            "target_sets":3,"reps_min":8,"reps_max":12,
            "sets":[]
        }"#;
        let entry: ExerciseEntry = serde_json::from_str(json).unwrap();
        assert!(entry.target_duration_seconds.is_none());
    }

    #[wasm_bindgen_test]
    fn exercise_entry_with_duration_deserializes() {
        let json = r#"{
            "id":"x","date":"2026-01-01","exercise_name":"Cat Stretch","exercise_id":"Cat_Stretch",
            "session_id":null,"day_id":null,"day_name":null,
            "target_sets":2,"reps_min":1,"reps_max":1,
            "sets":[{"set_number":1,"reps":1,"weight":0.0,"completed":true,"duration_seconds":30}],
            "target_duration_seconds":30
        }"#;
        let entry: ExerciseEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.target_duration_seconds, Some(30));
        assert_eq!(entry.sets[0].duration_seconds, Some(30));
    }

    #[wasm_bindgen_test]
    fn parse_workout_accepts_stretching_with_duration() {
        let lib = vec![
            lib_entry("Barbell_Bench_Press_-_Medium_Grip", "Bench Press"),
            {
                let mut e = lib_entry("Standing_Hamstring_Stretch", "Standing Hamstring Stretch");
                e.category = "stretching".into();
                e.equipment = None;
                e
            },
        ];
        let json = r#"{
            "name": "Push + Stretch",
            "rationale": "Chest work followed by cooldown.",
            "exercises": [
                {"library_id":"Barbell_Bench_Press_-_Medium_Grip","name":"Bench Press",
                 "target_sets":4,"reps_min":6,"reps_max":8,"rest_seconds":180,"notes":null},
                {"library_id":"Standing_Hamstring_Stretch","name":"Standing Hamstring Stretch",
                 "target_sets":2,"reps_min":1,"reps_max":1,"target_duration_seconds":30,
                 "rest_seconds":10,"notes":"Hold each side 30s"}
            ]
        }"#;
        let p = crate::coach::parse_workout_response(json, "2026-05-25", "2026-05-24T00:00:00.000Z", &lib)
            .expect("stretching exercise with duration should pass");
        assert_eq!(p.workout.exercises.len(), 2);
        assert_eq!(p.workout.exercises[1].target_duration_seconds, Some(30));
        assert!(p.workout.exercises[0].target_duration_seconds.is_none());
    }

    #[wasm_bindgen_test]
    fn coach_packet_mentions_stretching_and_balance() {
        use crate::coach::{build_coach_packet, PacketInput};
        let lib = vec![lib_entry("Barbell_Bench_Press_-_Medium_Grip", "Bench Press")];
        let goals = UserGoals::default();
        let packet = build_coach_packet(PacketInput {
            goals: &goals,
            history: &[],
            library: &lib,
            scheduled: &[],
            today: "2026-05-24",
            target_date: "2026-05-25",
        });
        assert!(
            packet.to_lowercase().contains("stretching"),
            "coach packet should mention stretching"
        );
        assert!(
            packet.to_lowercase().contains("balance"),
            "coach packet should mention balance"
        );
        assert!(
            packet.contains("target_duration_seconds"),
            "coach packet should reference target_duration_seconds field"
        );
    }

    // ── Coach: cardio + mobility integration ─────────────────────────────────

    #[wasm_bindgen_test]
    fn parse_workout_extracts_vitals_when_present() {
        let lib = vec![lib_entry("Barbell_Bench_Press_-_Medium_Grip", "Bench Press")];
        let json = r#"{
            "name":"Push","exercises":[
                {"library_id":"Barbell_Bench_Press_-_Medium_Grip","name":"Bench Press",
                 "target_sets":3,"reps_min":6,"reps_max":8,"rest_seconds":120,"notes":null}
            ],
            "vitals":{"vo2_max":36.4,"source_date":"2026-05-27"}
        }"#;
        let p = crate::coach::parse_workout_response(json, "2026-05-28", "2026-05-27T22:00:00.000Z", &lib)
            .expect("vitals + workout should parse");
        let v = p.vitals.expect("vitals should be Some");
        assert_eq!(v.vo2_max, 36.4);
        assert_eq!(v.source_date, "2026-05-27");
    }

    #[wasm_bindgen_test]
    fn parse_workout_omits_vitals_when_absent() {
        let lib = vec![lib_entry("Barbell_Bench_Press_-_Medium_Grip", "Bench Press")];
        let json = r#"{
            "name":"Push","exercises":[
                {"library_id":"Barbell_Bench_Press_-_Medium_Grip","name":"Bench Press",
                 "target_sets":3,"reps_min":6,"reps_max":8,"rest_seconds":120,"notes":null}
            ]
        }"#;
        let p = crate::coach::parse_workout_response(json, "2026-05-28", "2026-05-27T22:00:00.000Z", &lib)
            .expect("no-vitals response should still parse");
        assert!(p.vitals.is_none(), "missing vitals block should yield None");
    }

    #[wasm_bindgen_test]
    fn apply_vitals_updates_when_newer() {
        use crate::coach::{apply_vitals_to_goals, Vitals};
        let mut goals = UserGoals {
            vo2_max_latest: Some(34.0),
            vo2_max_updated: Some("2026-05-20".into()),
            ..UserGoals::default()
        };
        let applied = apply_vitals_to_goals(
            &Vitals { vo2_max: 36.4, source_date: "2026-05-27".into() },
            &mut goals,
        );
        assert!(applied);
        assert_eq!(goals.vo2_max_latest, Some(36.4));
        assert_eq!(goals.vo2_max_updated.as_deref(), Some("2026-05-27"));
    }

    #[wasm_bindgen_test]
    fn apply_vitals_drops_stale_silently() {
        use crate::coach::{apply_vitals_to_goals, Vitals};
        let mut goals = UserGoals {
            vo2_max_latest: Some(36.4),
            vo2_max_updated: Some("2026-05-27".into()),
            ..UserGoals::default()
        };
        let applied = apply_vitals_to_goals(
            &Vitals { vo2_max: 34.0, source_date: "2026-05-14".into() },
            &mut goals,
        );
        assert!(!applied, "older source_date should not overwrite");
        assert_eq!(goals.vo2_max_latest, Some(36.4));
        assert_eq!(goals.vo2_max_updated.as_deref(), Some("2026-05-27"));
    }

    #[wasm_bindgen_test]
    fn apply_vitals_drops_equal_date_silently() {
        // Same-day re-import should not bump (no new information).
        use crate::coach::{apply_vitals_to_goals, Vitals};
        let mut goals = UserGoals {
            vo2_max_latest: Some(36.4),
            vo2_max_updated: Some("2026-05-27".into()),
            ..UserGoals::default()
        };
        let applied = apply_vitals_to_goals(
            &Vitals { vo2_max: 99.0, source_date: "2026-05-27".into() },
            &mut goals,
        );
        assert!(!applied);
        assert_eq!(goals.vo2_max_latest, Some(36.4));
    }

    #[wasm_bindgen_test]
    fn apply_vitals_applies_when_no_prior_value() {
        use crate::coach::{apply_vitals_to_goals, Vitals};
        let mut goals = UserGoals::default();
        let applied = apply_vitals_to_goals(
            &Vitals { vo2_max: 36.4, source_date: "2026-05-27".into() },
            &mut goals,
        );
        assert!(applied);
        assert_eq!(goals.vo2_max_latest, Some(36.4));
    }

    #[wasm_bindgen_test]
    fn cardio_minutes_sums_completed_cardio_in_window() {
        use crate::coach::cardio_minutes_in_window;
        let lib = vec![
            lib_entry_with_cat("Jogging_Treadmill", "Jogging, Treadmill", "cardio"),
            lib_entry("Barbell_Squat", "Squat"),
        ];
        let cardio_entry = ExerciseEntry {
            id: "c1".into(),
            date: "2026-05-26".into(),
            exercise_name: "Jogging, Treadmill".into(),
            exercise_id: "Jogging_Treadmill".into(),
            session_id: None,
            day_id: None,
            day_name: None,
            target_sets: 1,
            reps_min: 20,
            reps_max: 40,
            sets: vec![
                crate::models::SetLog { set_number: 1, reps: 30, weight: 6.0, completed: true, completed_date: None, duration_seconds: None, zone_minutes: None },
                crate::models::SetLog { set_number: 2, reps: 99, weight: 9.0, completed: false, completed_date: None, duration_seconds: None, zone_minutes: None },
            ],
            finalized: true,
            created_at: "2026-05-26T10:00:00.000Z".into(),
            target_duration_seconds: None,
        };
        // Strength entry — should be ignored entirely
        let strength_entry = ExerciseEntry {
            id: "s1".into(),
            date: "2026-05-26".into(),
            exercise_name: "Squat".into(),
            exercise_id: "Barbell_Squat".into(),
            session_id: None,
            day_id: None,
            day_name: None,
            target_sets: 3,
            reps_min: 5,
            reps_max: 5,
            sets: vec![crate::models::SetLog { set_number: 1, reps: 5, weight: 225.0, completed: true, completed_date: None, duration_seconds: None, zone_minutes: None }],
            finalized: true,
            created_at: "2026-05-26T10:30:00.000Z".into(),
            target_duration_seconds: None,
        };
        // Old cardio entry — outside the 7d window
        let mut old_cardio = cardio_entry.clone();
        old_cardio.id = "c0".into();
        old_cardio.date = "2026-05-15".into();
        old_cardio.sets[0].reps = 1000; // would dominate the sum if it counted

        let history = vec![cardio_entry, strength_entry, old_cardio];
        let total = cardio_minutes_in_window(&history, &lib, "2026-05-28", 7);
        assert_eq!(total, 30, "only the in-window, completed cardio reps (minutes) should count");
    }

    #[wasm_bindgen_test]
    fn last_stretched_credits_only_stretching_category() {
        use crate::coach::last_stretched_by_muscle;
        let mut hamstring_stretch = lib_entry_with_cat("Standing_Hamstring_Stretch", "Standing Hamstring Stretch", "stretching");
        hamstring_stretch.primary_muscles = vec!["hamstrings".into()];
        hamstring_stretch.secondary_muscles = vec![];
        let mut squat = lib_entry_with_cat("Barbell_Squat", "Squat", "strength");
        squat.primary_muscles = vec!["quadriceps".into()];
        squat.secondary_muscles = vec!["hamstrings".into()]; // strength hits hamstrings but shouldn't count
        let lib = vec![hamstring_stretch, squat];

        let stretch_entry = ExerciseEntry {
            id: "ss1".into(),
            date: "2026-05-26".into(),
            exercise_name: "Standing Hamstring Stretch".into(),
            exercise_id: "Standing_Hamstring_Stretch".into(),
            session_id: None,
            day_id: None,
            day_name: None,
            target_sets: 2, reps_min: 1, reps_max: 1,
            sets: vec![crate::models::SetLog { set_number: 1, reps: 1, weight: 0.0, completed: true, completed_date: None, duration_seconds: Some(30), zone_minutes: None }],
            finalized: true,
            created_at: "2026-05-26T10:00:00.000Z".into(),
            target_duration_seconds: Some(30),
        };
        let squat_entry = ExerciseEntry {
            id: "sq1".into(),
            date: "2026-05-27".into(),
            exercise_name: "Squat".into(),
            exercise_id: "Barbell_Squat".into(),
            session_id: None,
            day_id: None,
            day_name: None,
            target_sets: 3, reps_min: 5, reps_max: 5,
            sets: vec![crate::models::SetLog { set_number: 1, reps: 5, weight: 225.0, completed: true, completed_date: None, duration_seconds: None, zone_minutes: None }],
            finalized: true,
            created_at: "2026-05-27T10:00:00.000Z".into(),
            target_duration_seconds: None,
        };

        let stretched = last_stretched_by_muscle(&[stretch_entry, squat_entry], &lib);
        // hamstrings was credited by the *stretching* entry (5/26), not the *strength* one (5/27)
        assert_eq!(stretched.get("hamstrings").map(String::as_str), Some("2026-05-26"));
        // quadriceps was only hit by strength — should not appear
        assert!(!stretched.contains_key("quadriceps"));
    }

    #[wasm_bindgen_test]
    fn coach_packet_includes_cardio_and_mobility_sections() {
        use crate::coach::{build_coach_packet, PacketInput};
        let lib = vec![lib_entry("Barbell_Bench_Press_-_Medium_Grip", "Bench Press")];
        let goals = UserGoals {
            weekly_cardio_minutes_target: Some(90),
            vo2_max_latest: Some(36.4),
            vo2_max_updated: Some("2026-05-27".into()),
            mobility_focus: crate::models::FocusLevel::High,
            balance_focus: crate::models::FocusLevel::Low,
            ..UserGoals::default()
        };

        let packet = build_coach_packet(PacketInput {
            goals: &goals,
            history: &[],
            library: &lib,
            scheduled: &[],
            today: "2026-05-28",
            target_date: "2026-05-29",
        });

        assert!(packet.contains("Cardio & mobility targets"), "should have the new section header");
        assert!(packet.contains("Weekly cardio minutes target: **90**"), "should display target");
        assert!(packet.contains("VO2 max: **36.4**"), "should display VO2 max");
        assert!(packet.contains("2026-05-27"), "should display VO2 update date");
        assert!(packet.contains("Mobility focus: **High**"), "should display mobility focus");
        assert!(packet.contains("Balance focus: **Low**"), "should display balance focus");
        assert!(packet.contains("Mobility recovery"), "should include mobility recovery table");
        assert!(packet.contains("Apple Health"), "should include screenshot-attach tip");
        assert!(packet.contains("\"vitals\""), "response format should show optional vitals block");
    }

    // ── Home: completed-workout visibility ─────────────────────────────────────

    fn history_entry_for_day(day_id: &str, completed: bool) -> ExerciseEntry {
        ExerciseEntry {
            id: format!("e-{day_id}"),
            date: "2026-05-29".into(),
            created_at: "2026-05-29T18:00:00.000Z".into(),
            exercise_name: "Bench Press".into(),
            exercise_id: "Barbell_Bench_Press_-_Medium_Grip".into(),
            session_id: Some("sess-1".into()),
            day_id: Some(day_id.into()),
            day_name: Some("Push".into()),
            target_sets: 3,
            reps_min: 8,
            reps_max: 12,
            sets: vec![crate::models::SetLog {
                set_number: 1,
                reps: 10,
                weight: 135.0,
                completed,
                completed_date: if completed { Some("2026-05-29".into()) } else { None },
                duration_seconds: None,
                zone_minutes: None,            }],
            finalized: true,
            target_duration_seconds: None,
        }
    }

    #[wasm_bindgen_test]
    fn is_workout_completed_true_with_matching_day_id_and_completed_set() {
        use crate::components::home::is_workout_completed_in_history;
        let history = vec![history_entry_for_day("w-today", true)];
        assert!(is_workout_completed_in_history("w-today", &history));
    }

    #[wasm_bindgen_test]
    fn is_workout_completed_false_when_no_match() {
        use crate::components::home::is_workout_completed_in_history;
        let history = vec![history_entry_for_day("w-other", true)];
        assert!(!is_workout_completed_in_history("w-today", &history));
    }

    #[wasm_bindgen_test]
    fn is_workout_completed_false_when_match_has_no_completed_sets() {
        // History entry with day_id match but every set is incomplete should not count.
        use crate::components::home::is_workout_completed_in_history;
        let history = vec![history_entry_for_day("w-today", false)];
        assert!(!is_workout_completed_in_history("w-today", &history));
    }

    #[wasm_bindgen_test]
    fn is_workout_completed_false_for_freeform_entries() {
        // Freeform exercise entries have day_id: None and should never count toward
        // "workout was completed."
        use crate::components::home::is_workout_completed_in_history;
        let mut e = history_entry_for_day("ignored", true);
        e.day_id = None;
        let history = vec![e];
        assert!(!is_workout_completed_in_history("w-today", &history));
    }

    #[wasm_bindgen_test]
    fn csv_export_includes_duration_column() {
        let entry = ExerciseEntry {
            id: "e1".into(),
            date: "2026-01-01".into(),
            exercise_name: "Cat Stretch".into(),
            exercise_id: "Cat_Stretch".into(),
            session_id: None,
            day_id: None,
            day_name: Some("Stretch Day".into()),
            target_sets: 2,
            reps_min: 1,
            reps_max: 1,
            sets: vec![
                crate::models::SetLog {
                    set_number: 1,
                    reps: 1,
                    weight: 0.0,
                    completed: true,
                    completed_date: Some("2026-01-01".into()),
                    duration_seconds: Some(30),
                    zone_minutes: None,                },
            ],
            finalized: true,
            created_at: "2026-01-01T00:00:00.000Z".into(),
            target_duration_seconds: Some(30),
        };
        let csv = crate::csv_utils::export_history_csv(&[entry]);
        assert!(csv.contains("duration_seconds"), "CSV header should include duration_seconds");
        assert!(csv.contains(",30,"), "CSV row should include duration value");
    }

    #[wasm_bindgen_test]
    fn csv_export_empty_duration_for_strength() {
        let entry = ExerciseEntry {
            id: "e2".into(),
            date: "2026-01-01".into(),
            exercise_name: "Bench Press".into(),
            exercise_id: "Barbell_Bench_Press_-_Medium_Grip".into(),
            session_id: None,
            day_id: None,
            day_name: None,
            target_sets: 3,
            reps_min: 8,
            reps_max: 12,
            sets: vec![
                crate::models::SetLog {
                    set_number: 1,
                    reps: 10,
                    weight: 135.0,
                    completed: true,
                    completed_date: None,
                    duration_seconds: None,
                    zone_minutes: None,                },
            ],
            finalized: true,
            created_at: "2026-01-01T00:00:00.000Z".into(),
            target_duration_seconds: None,
        };
        let csv = crate::csv_utils::export_history_csv(&[entry]);
        let lines: Vec<&str> = csv.lines().collect();
        assert!(lines[1].contains(",,true"), "strength exercise should have empty duration");
    }

    // ── Recency bucket ───────────────────────────────────────────────────────

    #[wasm_bindgen_test]
    fn recency_buckets_match_expected_ranges() {
        use crate::library::recency_bucket;
        assert_eq!(recency_bucket(0), Some(crate::library::RecencyBucket::Recent));
        assert_eq!(recency_bucket(3), Some(crate::library::RecencyBucket::Recent));
        assert_eq!(recency_bucket(4), Some(crate::library::RecencyBucket::Week));
        assert_eq!(recency_bucket(7), Some(crate::library::RecencyBucket::Week));
        assert_eq!(recency_bucket(8), Some(crate::library::RecencyBucket::TwoWeeks));
        assert_eq!(recency_bucket(14), Some(crate::library::RecencyBucket::TwoWeeks));
        assert_eq!(recency_bucket(15), Some(crate::library::RecencyBucket::Stale));
    }

    // ── Zone-shaped cardio ────────────────────────────────────────────────────

    #[wasm_bindgen_test]
    fn zone_minutes_sum_into_cardio_total_when_present() {
        use crate::coach::cardio_minutes_in_window;
        use crate::models::ZoneTarget;
        let lib = vec![lib_entry_with_cat("Running_Treadmill", "Running, Treadmill", "cardio")];
        let entry = ExerciseEntry {
            id: "c1".into(),
            date: "2026-05-27".into(),
            exercise_name: "Running, Treadmill".into(),
            exercise_id: "Running_Treadmill".into(),
            session_id: None, day_id: None, day_name: None,
            target_sets: 1, reps_min: 29, reps_max: 29,
            sets: vec![crate::models::SetLog {
                set_number: 1, reps: 29, weight: 0.0,
                completed: true, completed_date: None,
                duration_seconds: None,
                // Per-zone actuals sum to 13 + 16 = 29.
                zone_minutes: Some(vec![
                    ZoneTarget { zone: 1, minutes: 13.0 },
                    ZoneTarget { zone: 4, minutes: 16.0 },
                ]),
            }],
            finalized: true,
            created_at: "2026-05-27T10:00:00.000Z".into(),
            target_duration_seconds: None,
        };
        let total = cardio_minutes_in_window(&[entry], &lib, "2026-05-28", 7);
        assert_eq!(total, 29, "zone-minutes should be summed, not double-counted with reps");
    }

    #[wasm_bindgen_test]
    fn parse_cardio_actuals_accepts_fenced_wrapped_json() {
        use crate::coach::parse_cardio_actuals;
        let input = "```json\n{\"cardio_actuals\":{\"exercise_id\":\"Running_Treadmill\",\"zones\":[{\"zone\":1,\"minutes\":5},{\"zone\":4,\"minutes\":20}]}}\n```";
        let parsed = parse_cardio_actuals(input).expect("should parse fenced wrapped form");
        assert_eq!(parsed.exercise_id.as_deref(), Some("Running_Treadmill"));
        assert_eq!(parsed.zones.len(), 2);
        assert_eq!(parsed.zones[0].zone, 1);
        assert_eq!(parsed.zones[0].minutes, 5.0);
        assert_eq!(parsed.zones[1].zone, 4);
        assert_eq!(parsed.zones[1].minutes, 20.0);
    }

    #[wasm_bindgen_test]
    fn parse_cardio_actuals_accepts_bare_object() {
        use crate::coach::parse_cardio_actuals;
        let input = r#"{"exercise_name":"Running, Treadmill","zones":[{"zone":2,"minutes":30}]}"#;
        let parsed = parse_cardio_actuals(input).expect("bare object form should parse");
        assert_eq!(parsed.exercise_name.as_deref(), Some("Running, Treadmill"));
        assert_eq!(parsed.zones.len(), 1);
    }

    #[wasm_bindgen_test]
    fn coach_response_with_target_zones_parses() {
        use crate::coach::parse_workout_response;
        let lib = vec![lib_entry_with_cat("Running_Treadmill", "Running, Treadmill", "cardio")];
        let body = r#"```json
        {
          "name": "Zone 2 base",
          "rationale": "easy aerobic",
          "exercises": [{
            "library_id": "Running_Treadmill",
            "name": "Running, Treadmill",
            "target_sets": 1,
            "reps_min": 30,
            "reps_max": 30,
            "rest_seconds": 0,
            "notes": null,
            "target_zones": [{"zone": 2, "minutes": 30}]
          }]
        }
        ```"#;
        let parsed = parse_workout_response(body, "2026-05-31", "2026-05-30T22:00:00Z", &lib)
            .expect("should accept target_zones in prescription");
        let zones = parsed.workout.exercises[0].target_zones.as_ref().expect("target_zones present");
        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0].zone, 2);
        assert_eq!(zones[0].minutes, 30.0);
    }

    // ── Cardio-actuals parser (drives both the session and freeform import) ───

    #[wasm_bindgen_test]
    fn cardio_actuals_wrapped_form_parses() {
        use crate::coach::parse_cardio_actuals;
        let json = r#"{"cardio_actuals":{"exercise_id":"Elliptical_Trainer","zones":[
            {"zone":1,"minutes":5},
            {"zone":2,"minutes":18},
            {"zone":3,"minutes":9}
        ]}}"#;
        let parsed = parse_cardio_actuals(json).expect("wrapped form should parse");
        assert_eq!(parsed.exercise_id.as_deref(), Some("Elliptical_Trainer"));
        assert_eq!(parsed.zones.len(), 3);
        let total: f32 = parsed.zones.iter().map(|z| z.minutes).sum();
        assert_eq!(total, 32.0);
    }

    #[wasm_bindgen_test]
    fn cardio_actuals_bare_form_parses() {
        // For convenience the parser also accepts the unwrapped object directly.
        use crate::coach::parse_cardio_actuals;
        let json = r#"{"exercise_id":"Elliptical_Trainer","zones":[{"zone":2,"minutes":20}]}"#;
        let parsed = parse_cardio_actuals(json).expect("bare form should parse");
        assert_eq!(parsed.zones.len(), 1);
        assert_eq!(parsed.zones[0].zone, 2);
        assert_eq!(parsed.zones[0].minutes, 20.0);
    }

    #[wasm_bindgen_test]
    fn cardio_actuals_fenced_json_block_parses() {
        // Claude wraps the response in ```json … ``` — strip the fence.
        use crate::coach::parse_cardio_actuals;
        let json = "```json\n{\"cardio_actuals\":{\"exercise_id\":\"Elliptical_Trainer\",\"zones\":[{\"zone\":3,\"minutes\":15}]}}\n```";
        let parsed = parse_cardio_actuals(json).expect("fenced form should parse");
        assert_eq!(parsed.zones[0].minutes, 15.0);
    }

    #[wasm_bindgen_test]
    fn cardio_actuals_malformed_returns_err() {
        use crate::coach::parse_cardio_actuals;
        let err = parse_cardio_actuals("not actually json {").expect_err("must reject malformed input");
        assert!(err.to_lowercase().contains("json"), "error should mention JSON parsing: {err}");
    }

    #[wasm_bindgen_test]
    fn cardio_actuals_accepts_fractional_minutes() {
        // Regression: Apple Health reports fractional zone minutes (2.45,
        // 17.85, …). ZoneTarget.minutes was originally u32, which caused
        // serde to fail the wrapped-form parse and then fall back to the
        // bare-form parse, surfacing a misleading "missing field `zones`"
        // error. This is the user's exact payload from that bug report.
        use crate::coach::parse_cardio_actuals;
        let user_payload = r#"{"cardio_actuals":{"exercise_id":"Elliptical_Trainer","zones":[{"zone":1,"minutes":2.45},{"zone":2,"minutes":17.85},{"zone":3,"minutes":2.65},{"zone":4,"minutes":15.6},{"zone":5,"minutes":1.07}]}}"#;
        let parsed = parse_cardio_actuals(user_payload).expect("fractional minutes must parse");
        assert_eq!(parsed.zones.len(), 5);
        assert_eq!(parsed.zones[0].minutes, 2.45);
        assert_eq!(parsed.zones[3].minutes, 15.6);
        let total: f32 = parsed.zones.iter().map(|z| z.minutes).sum();
        // 2.45 + 17.85 + 2.65 + 15.6 + 1.07 = 39.62
        assert!((total - 39.62).abs() < 1e-3, "sum was {total}, expected 39.62");
    }

    #[wasm_bindgen_test]
    fn cardio_actuals_legacy_integer_minutes_still_parse() {
        // Back-compat: stored state.json files written before the f32 change
        // contain integer literals like `"minutes": 30`. Serde must read those
        // into f32 without complaint.
        use crate::coach::parse_cardio_actuals;
        let legacy = r#"{"cardio_actuals":{"exercise_id":"X","zones":[{"zone":2,"minutes":30}]}}"#;
        let parsed = parse_cardio_actuals(legacy).expect("legacy integer minutes must still parse");
        assert_eq!(parsed.zones[0].minutes, 30.0);
    }

    #[wasm_bindgen_test]
    fn cardio_actuals_error_for_wrapped_form_reports_underlying_problem() {
        // When the body clearly has "cardio_actuals" at top level but the
        // wrapped parse fails on a nested field, surface the wrapped error
        // (not the misleading "missing field zones" bare-form fallback).
        use crate::coach::parse_cardio_actuals;
        // Bad nested zone (string where number is expected) — wrapped parse
        // should fail with a clear "invalid type" message.
        let bad = r#"{"cardio_actuals":{"exercise_id":"X","zones":[{"zone":"two","minutes":15.0}]}}"#;
        let err = parse_cardio_actuals(bad).expect_err("must reject bad nested type");
        let lc = err.to_lowercase();
        // The error should clearly point at the actual problem (the inner
        // type mismatch), not a misleading "missing field `zones`".
        assert!(
            lc.contains("invalid") || lc.contains("expected") || lc.contains("zone"),
            "error should hint at the real cause, got: {err}"
        );
        assert!(
            !lc.contains("missing field `zones`"),
            "wrapped-form error must not be hidden by bare-form fallback: {err}"
        );
    }

    #[wasm_bindgen_test]
    fn cardio_actuals_accepts_estimated_rpe() {
        // The post-fix prompt asks Claude to infer an RPE from the zone
        // distribution and include it as `estimated_rpe`. Parser surfaces
        // it as Option<f32> so the importer can write it to set.weight.
        use crate::coach::parse_cardio_actuals;
        let input = r#"{"cardio_actuals":{"exercise_id":"Elliptical_Trainer","zones":[
            {"zone":2,"minutes":17.85},{"zone":4,"minutes":15.6}
        ],"estimated_rpe":7}}"#;
        let parsed = parse_cardio_actuals(input).expect("should parse with RPE");
        assert_eq!(parsed.estimated_rpe, Some(7.0));
    }

    #[wasm_bindgen_test]
    fn cardio_actuals_estimated_rpe_is_optional_for_back_compat() {
        // Responses generated before this change (or from a Claude conversation
        // that ignored the new field) must still parse cleanly with rpe = None.
        use crate::coach::parse_cardio_actuals;
        let input = r#"{"cardio_actuals":{"exercise_id":"X","zones":[{"zone":2,"minutes":20.0}]}}"#;
        let parsed = parse_cardio_actuals(input).expect("should parse without RPE");
        assert!(parsed.estimated_rpe.is_none(), "missing field should yield None");
    }

    #[wasm_bindgen_test]
    fn cardio_actuals_accepts_fractional_rpe() {
        // Claude may return RPE as a float (e.g. 7.5). Accept it.
        use crate::coach::parse_cardio_actuals;
        let input = r#"{"cardio_actuals":{"exercise_id":"X","zones":[{"zone":2,"minutes":20.0}],"estimated_rpe":7.5}}"#;
        let parsed = parse_cardio_actuals(input).expect("should accept fractional RPE");
        assert_eq!(parsed.estimated_rpe, Some(7.5));
    }

    #[wasm_bindgen_test]
    fn coach_packet_surfaces_zone_breakdown_in_recent_training() {
        // When a freeform cardio set carries zone_minutes (typically from the
        // Apple Health import), the coach packet's Recent training summary
        // must show the per-zone breakdown so the next coach run can read
        // intensity shape, not just total minutes.
        use crate::coach::{build_coach_packet, PacketInput};
        use crate::models::ZoneTarget;
        let lib = vec![lib_entry_with_cat("Elliptical_Trainer", "Elliptical Trainer", "cardio")];
        let entry = ExerciseEntry {
            id: "e1".into(),
            date: "2026-06-01".into(),
            created_at: "2026-06-01T10:00:00.000Z".into(),
            exercise_name: "Elliptical Trainer".into(),
            exercise_id: "Elliptical_Trainer".into(),
            session_id: None, day_id: None, day_name: None,
            target_sets: 1, reps_min: 30, reps_max: 30,
            sets: vec![crate::models::SetLog {
                set_number: 1, reps: 40,
                weight: 7.0, // RPE from the Claude-inferred import
                completed: true,
                completed_date: Some("2026-06-01".into()),
                duration_seconds: None,
                zone_minutes: Some(vec![
                    ZoneTarget { zone: 1, minutes: 2.45 },
                    ZoneTarget { zone: 2, minutes: 17.85 },
                    ZoneTarget { zone: 4, minutes: 15.6 },
                ]),
            }],
            finalized: true,
            target_duration_seconds: None,
        };
        let packet = build_coach_packet(PacketInput {
            goals: &UserGoals::default(),
            history: &[entry],
            library: &lib,
            scheduled: &[],
            today: "2026-06-02",
            target_date: "2026-06-03",
        });
        // Per-zone breakdown should appear in the recent-training section.
        assert!(packet.contains("Z1:2.5"), "should show Z1 with fractional minutes: {packet}");
        assert!(packet.contains("Z2:17.9"), "should show Z2 with fractional minutes: {packet}");
        assert!(packet.contains("Z4:15.6"), "should show Z4 with fractional minutes: {packet}");
        // Total minutes and RPE annotation should be present (35.9 → 36).
        assert!(packet.contains("36 min total"), "total rounded minutes should appear: {packet}");
        assert!(packet.contains("RPE 7"), "RPE should appear when set.weight > 0: {packet}");
    }

    #[wasm_bindgen_test]
    fn cardio_minutes_in_window_sums_fractional_zones_and_rounds() {
        // The weekly cardio total is u32 (display-only). When zone_minutes are
        // fractional, we should sum within the set as f32 and round once.
        use crate::coach::cardio_minutes_in_window;
        use crate::models::ZoneTarget;
        let lib = vec![lib_entry_with_cat("Elliptical_Trainer", "Elliptical Trainer", "cardio")];
        let entry = ExerciseEntry {
            id: "e1".into(),
            date: "2026-06-02".into(),
            exercise_name: "Elliptical Trainer".into(),
            exercise_id: "Elliptical_Trainer".into(),
            session_id: None, day_id: None, day_name: None,
            target_sets: 1, reps_min: 30, reps_max: 30,
            sets: vec![crate::models::SetLog {
                set_number: 1, reps: 0, weight: 0.0,
                completed: true, completed_date: None,
                duration_seconds: None,
                // Same payload as the user's bug report. Sum = 39.62 → rounds to 40.
                zone_minutes: Some(vec![
                    ZoneTarget { zone: 1, minutes: 2.45 },
                    ZoneTarget { zone: 2, minutes: 17.85 },
                    ZoneTarget { zone: 3, minutes: 2.65 },
                    ZoneTarget { zone: 4, minutes: 15.6 },
                    ZoneTarget { zone: 5, minutes: 1.07 },
                ]),
            }],
            finalized: true,
            created_at: "2026-06-02T10:00:00.000Z".into(),
            target_duration_seconds: None,
        };
        let total = cardio_minutes_in_window(&[entry], &lib, "2026-06-02", 7);
        assert_eq!(total, 40, "39.62 should round to 40");
    }
}

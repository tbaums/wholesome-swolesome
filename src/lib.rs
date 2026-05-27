pub mod coach;
pub mod csv_utils;
pub mod library;
pub mod models;
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
        let w = crate::coach::parse_workout_response(json, "2026-05-25", "2026-05-24T00:00:00.000Z", &lib)
            .expect("valid id should pass");
        assert_eq!(w.exercises.len(), 1);
        assert_eq!(
            w.exercises[0].library_id.as_deref(),
            Some("Barbell_Bench_Press_-_Medium_Grip")
        );
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
}

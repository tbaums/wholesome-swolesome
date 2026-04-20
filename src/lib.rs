pub mod csv_utils;
pub mod models;
pub mod seed;

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
}

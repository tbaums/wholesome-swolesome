use crate::models::{Exercise, ExerciseCategory, WorkoutDay, WorkoutPlan};

/// Builds the default workout plan from the workout_plan.csv data.
pub fn default_plan() -> WorkoutPlan {
    WorkoutPlan {
        days: vec![
            WorkoutDay {
                id: "d1".into(),
                name: "Lower A: Glute Tension".into(),
                exercises: vec![
                    ex("d1-e1", "Hip thrust", 4, 6, 10, ExerciseCategory::Main, None),
                    ex("d1-e2", "Bulgarian split squat", 3, 8, 10, ExerciseCategory::Main, Some("per side")),
                    ex("d1-e3", "RDL", 3, 6, 10, ExerciseCategory::Main, None),
                    ex("d1-e4", "Kickback", 3, 10, 15, ExerciseCategory::Main, None),
                    ex("d1-e5", "Hip abduction", 3, 15, 25, ExerciseCategory::Main, None),
                    ex("d1-e6", "Machine/cable crunch", 3, 10, 15, ExerciseCategory::Core, None),
                    ex("d1-e7", "Pallof press", 3, 10, 15, ExerciseCategory::Core, Some("per side")),
                ],
            },
            WorkoutDay {
                id: "d2".into(),
                name: "Upper A: Maintenance".into(),
                exercises: vec![
                    ex("d2-e1", "Pulldown", 3, 8, 12, ExerciseCategory::Main, None),
                    ex("d2-e2", "Seated row", 3, 8, 12, ExerciseCategory::Main, None),
                    ex("d2-e3", "Rear delt fly", 3, 12, 20, ExerciseCategory::Main, None),
                    ex("d2-e4", "Light chest press", 2, 10, 15, ExerciseCategory::Main, None),
                    ex("d2-e5", "Lateral raise", 2, 12, 20, ExerciseCategory::Main, None),
                    ex("d2-e6", "Hanging knee raise", 3, 8, 15, ExerciseCategory::Core, None),
                    ex("d2-e7", "Suitcase carry", 3, 10, 15, ExerciseCategory::Core, Some("per side")),
                ],
            },
            WorkoutDay {
                id: "d3".into(),
                name: "Lower B: Glute Width".into(),
                exercises: vec![
                    ex("d3-e1", "Hip thrust", 3, 10, 15, ExerciseCategory::Main, None),
                    ex("d3-e2", "Step-up", 3, 8, 12, ExerciseCategory::Main, Some("per side")),
                    ex("d3-e3", "Hip abduction", 4, 15, 30, ExerciseCategory::Main, None),
                    ex("d3-e4", "Long-stride lunge", 3, 10, 10, ExerciseCategory::Main, Some("per side")),
                    ex("d3-e5", "Pull-through", 3, 12, 15, ExerciseCategory::Main, None),
                    ex("d3-e6", "Incline/decline sit-up", 3, 8, 15, ExerciseCategory::Core, None),
                    ex("d3-e7", "Cable chop", 3, 10, 15, ExerciseCategory::Core, Some("per side")),
                ],
            },
            WorkoutDay {
                id: "d4".into(),
                name: "Recovery / Aerobic Base".into(),
                exercises: vec![
                    ex("d4-e1", "Pallof hold", 3, 20, 30, ExerciseCategory::Core, Some("per side, seconds")),
                    ex("d4-e2", "Machine crunch", 2, 12, 20, ExerciseCategory::Core, None),
                    ex("d4-e3", "Zone 2 cardio", 1, 20, 30, ExerciseCategory::Cardio, Some("minutes")),
                ],
            },
            WorkoutDay {
                id: "d5".into(),
                name: "Lower C: Glute + Hamstring".into(),
                exercises: vec![
                    ex("d5-e1", "RDL or 45° back extension", 4, 6, 10, ExerciseCategory::Main, None),
                    ex("d5-e2", "Reverse/walking lunge", 3, 8, 12, ExerciseCategory::Main, Some("per side")),
                    ex("d5-e3", "Kickback", 3, 12, 15, ExerciseCategory::Main, None),
                    ex("d5-e4", "Hip abduction", 3, 15, 25, ExerciseCategory::Main, None),
                    ex("d5-e5", "Leg curl", 3, 8, 15, ExerciseCategory::Main, None),
                    ex("d5-e6", "Machine crunch", 3, 10, 15, ExerciseCategory::Core, None),
                    ex("d5-e7", "Suitcase carry", 3, 10, 15, ExerciseCategory::Core, Some("per side")),
                ],
            },
            WorkoutDay {
                id: "d6".into(),
                name: "Upper B: Maintenance + Intervals".into(),
                exercises: vec![
                    ex("d6-e1", "Pulldown", 3, 8, 12, ExerciseCategory::Main, None),
                    ex("d6-e2", "Row", 3, 8, 12, ExerciseCategory::Main, None),
                    ex("d6-e3", "Rear delt fly", 3, 12, 20, ExerciseCategory::Main, None),
                    ex("d6-e4", "Bicep curl", 2, 10, 15, ExerciseCategory::Main, None),
                    ex("d6-e5", "Tricep pressdown", 2, 10, 15, ExerciseCategory::Main, None),
                    ex("d6-e6", "Hanging knee raise", 3, 8, 15, ExerciseCategory::Core, None),
                    ex("d6-e7", "Anti-rotation press", 2, 10, 15, ExerciseCategory::Core, Some("per side")),
                    ex("d6-e8", "Intervals", 1, 10, 15, ExerciseCategory::Cardio, Some("minutes")),
                ],
            },
            WorkoutDay {
                id: "d7".into(),
                name: "Whole Body: Light / Polish".into(),
                exercises: vec![
                    ex("d7-e1", "Hip thrust or glute bridge", 3, 12, 15, ExerciseCategory::Main, None),
                    ex("d7-e2", "Leg press", 3, 10, 15, ExerciseCategory::Main, None),
                    ex("d7-e3", "Upper-back pull", 2, 10, 15, ExerciseCategory::Main, None),
                    ex("d7-e4", "Light press (optional)", 2, 10, 15, ExerciseCategory::Main, None),
                    ex("d7-e5", "Machine crunch", 3, 12, 20, ExerciseCategory::Core, None),
                    ex("d7-e6", "Pallof or cable chop", 2, 10, 15, ExerciseCategory::Core, Some("per side")),
                ],
            },
        ],
    }
}

fn ex(
    id: &str,
    name: &str,
    sets: u32,
    reps_min: u32,
    reps_max: u32,
    category: ExerciseCategory,
    notes: Option<&str>,
) -> Exercise {
    Exercise {
        id: id.into(),
        name: name.into(),
        target_sets: sets,
        reps_min,
        reps_max,
        category,
        notes: notes.map(str::to_string),
    }
}

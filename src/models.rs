use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkoutPlan {
    pub days: Vec<WorkoutDay>,
}

impl Default for WorkoutPlan {
    fn default() -> Self {
        crate::seed::default_plan()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkoutDay {
    pub id: String,
    pub name: String,
    pub exercises: Vec<Exercise>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Exercise {
    pub id: String,
    pub name: String,
    pub target_sets: u32,
    pub reps_min: u32,
    pub reps_max: u32,
    pub category: ExerciseCategory,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ExerciseCategory {
    #[default]
    Main,
    Core,
    Cardio,
}

impl ExerciseCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Main => "Main",
            Self::Core => "Core",
            Self::Cardio => "Cardio",
        }
    }
    #[allow(dead_code)]
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Main => "pill pill-main",
            Self::Core => "pill pill-core",
            Self::Cardio => "pill pill-cardio",
        }
    }
}

/// A completed (or in-progress) workout session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkoutSession {
    pub id: String,
    pub date: String, // "YYYY-MM-DD"
    pub day_id: String,
    pub day_name: String,
    pub exercise_logs: Vec<ExerciseLog>,
    pub is_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExerciseLog {
    pub exercise_id: String,
    pub exercise_name: String,
    pub target_sets: u32,
    pub reps_min: u32,
    pub reps_max: u32,
    pub sets: Vec<SetLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SetLog {
    pub set_number: u32,
    pub reps: u32,
    pub weight_lbs: f32,
    pub completed: bool,
}

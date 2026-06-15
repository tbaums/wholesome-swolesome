//! Native CLI that emits the coach brief — byte-identical to the in-app Coach
//! Brief — from a `state.json` + the exercise library. The single source of
//! truth for the brief is `coach::build_coach_packet`; both the in-app view and
//! the nightly coach consume this so the coaching logic lives in one place (#38).
//!
//! Native-only: it reads files and writes stdout, neither of which exists on
//! wasm32. On wasm the binary is an empty stub so the crate still builds for the
//! app target.
//!
//! Usage:
//!   coach-brief <state.json> <exercises.json> <today YYYY-MM-DD> <target YYYY-MM-DD>

#[cfg(not(target_arch = "wasm32"))]
mod cli {
    use std::process::exit;

    use wholesome_swolesome::coach::{build_coach_packet, PacketInput};
    use wholesome_swolesome::models::{ExerciseEntry, LibraryExercise, ScheduledWorkout, UserGoals};

    fn die(msg: &str) -> ! {
        eprintln!("coach-brief: {msg}");
        exit(1);
    }

    pub fn run() {
        let args: Vec<String> = std::env::args().collect();
        if args.len() != 5 {
            die("usage: coach-brief <state.json> <exercises.json> <today YYYY-MM-DD> <target YYYY-MM-DD>");
        }
        let (state_path, lib_path, today, target) = (&args[1], &args[2], &args[3], &args[4]);

        let state_raw = std::fs::read_to_string(state_path)
            .unwrap_or_else(|e| die(&format!("reading {state_path}: {e}")));
        let state: serde_json::Value = serde_json::from_str(&state_raw)
            .unwrap_or_else(|e| die(&format!("parsing {state_path}: {e}")));

        let goals: UserGoals =
            serde_json::from_value(state.get("goals").cloned().unwrap_or_default())
                .unwrap_or_else(|e| die(&format!("parsing goals: {e}")));
        let history: Vec<ExerciseEntry> =
            serde_json::from_value(state.get("exercise_history").cloned().unwrap_or_default())
                .unwrap_or_else(|e| die(&format!("parsing exercise_history: {e}")));
        let scheduled: Vec<ScheduledWorkout> =
            serde_json::from_value(state.get("scheduled_workouts").cloned().unwrap_or_default())
                .unwrap_or_else(|e| die(&format!("parsing scheduled_workouts: {e}")));

        let lib_raw = std::fs::read_to_string(lib_path)
            .unwrap_or_else(|e| die(&format!("reading {lib_path}: {e}")));
        let library: Vec<LibraryExercise> = serde_json::from_str(&lib_raw)
            .unwrap_or_else(|e| die(&format!("parsing {lib_path}: {e}")));

        let brief = build_coach_packet(PacketInput {
            goals: &goals,
            history: &history,
            library: &library,
            scheduled: &scheduled,
            today,
            target_date: target,
        });
        print!("{brief}");
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    cli::run();
}

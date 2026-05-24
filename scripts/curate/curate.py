#!/usr/bin/env python3
"""
Curate a focused subset of free-exercise-db into public/data/exercises.json
and copy referenced images to public/data/exercises/.

Selection goals:
  - All cardio entries (treadmill, elliptical, bike, etc.)
  - ~12-15 exercises per primary muscle group, biased toward common equipment,
    compound > isolation, beginner-friendly, with images
  - A small set of mobility/stretching staples
"""
from __future__ import annotations
import glob, json, os, shutil, sys
from pathlib import Path
from collections import defaultdict

SRC = Path("/tmp/free-exercise-db")
DST_ROOT = Path(__file__).resolve().parents[2] / "public" / "data"
DST_JSON = DST_ROOT / "exercises.json"
DST_IMG = DST_ROOT / "exercises"

# These primary-muscle anchors get filled first; we ensure coverage.
MUSCLES = [
    "chest", "lats", "middle back", "lower back", "traps", "shoulders",
    "biceps", "triceps", "forearms", "abdominals",
    "glutes", "quadriceps", "hamstrings", "calves",
    "abductors", "adductors", "neck",
]

# Per-muscle target counts. Bigger groups get more options.
TARGET_PER_MUSCLE = {
    "chest": 14, "lats": 12, "middle back": 10, "lower back": 8, "traps": 6,
    "shoulders": 14, "biceps": 10, "triceps": 10, "forearms": 6, "abdominals": 14,
    "glutes": 14, "quadriceps": 14, "hamstrings": 12, "calves": 8,
    "abductors": 4, "adductors": 4, "neck": 3,
}

# Equipment preference: lower number = better
EQUIPMENT_RANK = {
    "barbell": 1, "dumbbell": 1, "body only": 1, "machine": 2, "cable": 2,
    "kettlebells": 3, "bands": 4, "e-z curl bar": 5, "exercise ball": 5,
    "medicine ball": 6, "foam roll": 7, "other": 5, None: 6,
}

LEVEL_RANK = {"beginner": 1, "intermediate": 2, "expert": 3}

# Always include if present (foundational moves)
ALWAYS_INCLUDE = {
    # Lower body
    "Barbell_Squat", "Barbell_Deadlift", "Romanian_Deadlift", "Goblet_Squat",
    "Barbell_Hack_Squat", "Front_Box_Jump", "Bodyweight_Walking_Lunge",
    "Barbell_Walking_Lunge", "Barbell_Step_Ups", "Leg_Press",
    "Leg_Extensions", "Lying_Leg_Curls", "Seated_Leg_Curl",
    "Standing_Calf_Raises", "Seated_Calf_Raise", "Standing_Barbell_Calf_Raise",
    "Barbell_Glute_Bridge", "Single_Leg_Glute_Bridge", "Glute_Ham_Raise", "Step_Mill",
    # Push
    "Barbell_Bench_Press_-_Medium_Grip", "Dumbbell_Bench_Press",
    "Incline_Dumbbell_Press", "Decline_Dumbbell_Bench_Press",
    "Pushups", "Push-Ups_-_Close_Triceps_Position", "Decline_Push-Up",
    "Dips_-_Triceps_Version",
    "Cable_Crossover", "Dumbbell_Flyes",
    "Dumbbell_Shoulder_Press", "Seated_Dumbbell_Press",
    "Standing_Military_Press", "Arnold_Dumbbell_Press",
    "Side_Lateral_Raise", "Front_Dumbbell_Raise", "Bent_Over_Dumbbell_Rear_Delt_Raise_With_Head_On_Bench",
    # Pull
    "Pullups", "Chin-Up", "Bent_Over_Barbell_Row", "One-Arm_Dumbbell_Row",
    "Seated_Cable_Rows", "Wide-Grip_Lat_Pulldown", "Reverse_Grip_Bent-Over_Rows",
    "Face_Pull",
    "Barbell_Curl", "Dumbbell_Bicep_Curl", "Hammer_Curls",
    "Preacher_Curl", "EZ-Bar_Curl",
    # Triceps
    "Triceps_Pushdown", "Cable_Rope_Overhead_Triceps_Extension",
    "EZ-Bar_Skullcrusher", "Tricep_Dumbbell_Kickback",
    "Close-Grip_Barbell_Bench_Press",
    # Core
    "Plank", "Side_Bridge", "Hanging_Leg_Raise", "Russian_Twist",
    "Cable_Crunch", "Decline_Crunch", "Mountain_Climbers",
    "Pallof_Press", "Ab_Roller", "Air_Bike",
    # Plyo / metabolic
    "Box_Jump_Multiple_Response", "Front_Box_Jump",
    "Single-Leg_Lateral_Hop",
    # Forearms
    "Wrist_Roller", "Reverse_Barbell_Curl",
    # Mobility / warmups
    "Spider_Crawl",
}

# A few stretching staples (mobility)
STRETCH_INCLUDE = {
    "Standing_Hamstring_Stretch", "World_Greatest_Stretch", "Couch_Stretch",
    "Cobra_Stretch", "Cat_Stretch", "Childs_Pose",
    "Standing_Pectoral_Stretch", "Hip_Flexor_Stretch",
    "Standing_Calf_Stretch",
}


def score(ex: dict) -> tuple:
    """Lower is better."""
    eq = EQUIPMENT_RANK.get(ex.get("equipment"), 5)
    lvl = LEVEL_RANK.get(ex.get("level"), 3)
    mech = 0 if ex.get("mechanic") == "compound" else (1 if ex.get("mechanic") == "isolation" else 2)
    has_img = 0 if ex.get("images") else 1
    return (lvl, eq, mech, has_img, len(ex.get("name", "")))


def main() -> int:
    all_files = sorted(glob.glob(str(SRC / "exercises" / "*.json")))
    if not all_files:
        print(f"No exercises found at {SRC}", file=sys.stderr); return 1

    db = []
    for fp in all_files:
        with open(fp) as fh:
            db.append(json.load(fh))

    by_id = {e["id"]: e for e in db}
    chosen: dict[str, dict] = {}

    # 1) All cardio
    cardio = [e for e in db if e.get("category") == "cardio"]
    for e in cardio:
        chosen[e["id"]] = e

    # 2) Foundational always-include
    for eid in ALWAYS_INCLUDE:
        if eid in by_id:
            chosen[eid] = by_id[eid]

    # 3) Per-muscle fill (only consider strength/plyometrics; skip stretching/powerlifting/strongman/olympic for now)
    eligible = [
        e for e in db
        if e.get("category") in ("strength", "plyometrics")
        and e.get("level") in ("beginner", "intermediate")
        and (e.get("images") or [])
    ]

    per_muscle_have: dict[str, int] = defaultdict(int)
    for eid in chosen:
        for m in by_id[eid].get("primaryMuscles", []) or []:
            per_muscle_have[m] += 1

    # Sort by score globally
    eligible.sort(key=score)

    for m in MUSCLES:
        target = TARGET_PER_MUSCLE[m]
        if per_muscle_have[m] >= target:
            continue
        for e in eligible:
            if e["id"] in chosen: continue
            prim = e.get("primaryMuscles") or []
            if m not in prim: continue
            chosen[e["id"]] = e
            per_muscle_have[m] += 1
            for pm in prim:
                per_muscle_have[pm] += 1
            if per_muscle_have[m] >= target:
                break

    # 4) Stretching staples
    for eid in STRETCH_INCLUDE:
        if eid in by_id:
            chosen[eid] = by_id[eid]

    # Build output
    if DST_IMG.exists():
        shutil.rmtree(DST_IMG)
    DST_IMG.mkdir(parents=True, exist_ok=True)

    out = []
    for eid in sorted(chosen):
        e = chosen[eid]
        # Copy first 2 images per exercise (start + end position)
        new_images = []
        for img_path in (e.get("images") or [])[:2]:
            src_img = SRC / "exercises" / img_path
            if not src_img.exists(): continue
            dst_img = DST_IMG / img_path
            dst_img.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src_img, dst_img)
            new_images.append(img_path)

        # Shorten instructions: keep first 2-3 sentences worth, total <= 600 chars
        instructions = e.get("instructions") or []
        joined = " ".join(instructions).strip()
        if len(joined) > 700:
            # take first N steps that fit
            kept = []
            total = 0
            for step in instructions:
                if total + len(step) > 700: break
                kept.append(step)
                total += len(step)
            instructions = kept or instructions[:1]

        out.append({
            "id": e["id"],
            "name": e["name"],
            "force": e.get("force"),
            "level": e.get("level"),
            "mechanic": e.get("mechanic"),
            "equipment": e.get("equipment"),
            "primaryMuscles": e.get("primaryMuscles") or [],
            "secondaryMuscles": e.get("secondaryMuscles") or [],
            "instructions": instructions,
            "category": e.get("category"),
            "images": new_images,
        })

    DST_ROOT.mkdir(parents=True, exist_ok=True)
    with open(DST_JSON, "w") as fh:
        json.dump(out, fh, separators=(",", ":"))

    # Summary
    cats: dict[str, int] = defaultdict(int)
    eqs: dict[str, int] = defaultdict(int)
    muscles: dict[str, int] = defaultdict(int)
    for e in out:
        cats[e["category"]] += 1
        eqs[str(e["equipment"])] += 1
        for m in e["primaryMuscles"]: muscles[m] += 1

    print(f"Wrote {len(out)} exercises -> {DST_JSON}")
    print(f"Image dir: {DST_IMG}")
    print("Categories:", dict(cats))
    print("Equipment:", dict(eqs))
    print("Muscles (primary):", dict(sorted(muscles.items())))

    # Total size
    total = sum(p.stat().st_size for p in DST_IMG.rglob("*") if p.is_file())
    print(f"Images total: {total/1024/1024:.1f} MiB")
    json_size = DST_JSON.stat().st_size
    print(f"JSON size: {json_size/1024:.1f} KiB")

    return 0

if __name__ == "__main__":
    sys.exit(main())

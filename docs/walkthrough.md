<!--
  Curated screenshots → the walkthrough spec + raw file that produced each one.

  The harness (`npx playwright test --config=playwright.walkthrough.config.ts`)
  never writes to docs/walkthrough/. It writes raw, spec-numbered shots to the
  gitignored tests/playwright/screenshots/<spec>/. To refresh a curated image,
  run the harness and copy the raw file over the curated one, keeping the
  curated name:

    curated file                  spec                        raw file
    01-home-empty.png             walkthrough_hypertrophy     01_home_empty.png
    02-goals-and-cardio.png       walkthrough_hypertrophy     06_options_goals_filled.png
    03-library-detail.png         walkthrough_legday          02_library_detail_squat.png
    04-coach-brief-top.png        walkthrough_hypertrophy     08_coach_brief_top.png
    05-coach-brief-bottom.png     walkthrough_hypertrophy     09_coach_brief_bottom.png
    06-today-scheduled.png        walkthrough_hypertrophy     12_home_with_scheduled.png
    07-session-view.png           walkthrough_hypertrophy     14_session_exercise_open.png
    08-heatmap-after.png          walkthrough_hypertrophy     18_history_after_session.png
    09-home-done.png              walkthrough_hypertrophy     20_home_after_finish_done.png

  e.g. cp tests/playwright/screenshots/walkthrough_hypertrophy/01_home_empty.png \
          docs/walkthrough/01-home-empty.png

  The raw filenames are the spec's shot order, so adding or removing a `shot()`
  call renumbers everything after it — re-check this table when a spec changes.
  README.md embeds 06, 07 and 08; don't rename those.

  Capture modes (`helpers.ts`): everything is a `fullPageShot` — a fullPage
  capture with the fixed bottom nav and sticky header neutralised, since
  Chromium otherwise paints them at their *viewport* position, i.e. across the
  middle of a tall image, hiding whatever is behind them. 04 and 05 are the
  exception: they're the same document seen at two scroll positions, so they
  use `viewportShot` (a plain phone-screen capture, chrome where it belongs)
  and stay two distinct pictures.
-->

# Wholesome Swolesome — a tour

A mobile-first workout tracker that asks **Claude** to plan each day's session against
your goals, recent history, and per-muscle recovery state — then runs entirely in your
phone's browser with **GitHub as the database**. No servers, no accounts, no app-store
review cycle. This page is the 5-minute pitch.

**Live app:** <https://tbaums.github.io/wholesome-swolesome/> · **Repo:** <https://github.com/tbaums/wholesome-swolesome>

---

## A tour in 9 screenshots

### 1. Empty home

<img src="walkthrough/01-home-empty.png" width="320" alt="Empty Workout tab — 'No workout scheduled'">

You land on the *Workout* tab. There's no static plan, no calendar, no schedule. The
coach hasn't planned today's session yet, so you tap *Generate workout with Claude*
(or wait for the nightly agent to do it for you). Bottom nav: Workout · Library · Exercises · History.

### 2. Set your goals

<img src="walkthrough/02-goals-and-cardio.png" width="320" alt="Options page showing Training goals and Cardio & mobility cards">

One-time setup. Primary goal (Hypertrophy / Strength / Fat loss / Endurance / General
fitness), sessions per week, session minutes, available equipment, injuries to avoid,
freeform coaching notes. The two free-text fields aren't inline textareas: each is a
tappable preview card that clamps to three lines and opens a **full-screen editor**
(Cancel / Done) — on a phone, typing a paragraph into a 2-row box wedged between other
controls was miserable. Below: **Cardio & mobility** — weekly cardio minutes target,
latest VO2 max (with the date it's from), and Low/Standard/High focus levels for
mobility and balance. Everything below the Training-goals card is optional.

### 3. The exercise library

<img src="walkthrough/03-library-detail.png" width="320" alt="Library detail for Barbell Squat — photos, muscle silhouette, instructions">

290 exercises from the [free-exercise-db](https://github.com/yuhonas/free-exercise-db)
dataset — strength, cardio, stretching, balance, plyometrics. Each entry has photos,
primary/secondary muscle chips, a body silhouette colored to show what the lift hits,
and step-by-step instructions. The library is bundled at build time as a static JSON
file; no API calls.

### 4. The Coach Brief

<img src="walkthrough/04-coach-brief-top.png" width="320" alt="Coach Brief — markdown packet showing goals, cardio/mobility targets, and mobility recovery">

When you tap *Generate workout with Claude*, the app builds a markdown packet
containing everything Claude needs to plan a session: your goals, weekly cardio
minutes hit so far, current VO2 max, mobility/balance focus levels, days-since-
each-muscle-was-worked, days-since-each-muscle-was-stretched, recent training, and
the full exercise library inline (so Claude can only choose valid IDs).

Three things the brief is careful about, each the fix for a way the coach used to
go wrong:

- **"Worked" means strength/cardio/plyometric work only.** Stretching and balance
  drills don't count toward muscle recovery — they're tracked in the separate
  mobility table — so a week of hamstring stretches no longer reads as a week of
  hamstring training.
- **The recent-training rundown lists only sets you actually checked off.** Planned
  session titles are omitted entirely and exercises with zero completed sets are
  dropped, so a day *planned* as "Upper Pull" whose pull work never happened can't
  be mis-credited as done. Freeform drafts you never finalized still count — a
  checked-off set is real work.
- **No invented preferences.** The task rules tell Claude that your behavioral
  constraints are *only* what's written in the Goals section; it may not infer new
  ones from your history ("they seem to avoid floor work"). Cardio entries carry
  their per-zone minute breakdown and the RPE derived from it, so intensity is read
  off the real distribution rather than guessed.

### 5. Paste Claude's response

<img src="walkthrough/05-coach-brief-bottom.png" width="320" alt="Bottom of Coach Brief — paste-area for Claude's JSON response">

Copy the brief into any Claude conversation — Claude Code, the Claude mobile app,
claude.ai. (If you also paste an Apple Health VO2-max screenshot into the same
conversation, Claude will extract the reading and include it in the response.) You
get back a strict JSON object describing one workout. Paste it here, tap *Import workout*.
Cardio exercises in that JSON can carry per-zone heart-rate minutes rather than a flat
duration, which is what the Apple Health flow (step 7 and below) feeds back in.

### 6. Today's workout

<img src="walkthrough/06-today-scheduled.png" width="320" alt="Workout tab — today's TODAY card with planned exercises">

The imported workout shows up on the *TODAY* card. Title, the coach's one-paragraph
rationale, then the exercise list with target sets × rep ranges. Tap *Start workout →*.

### 7. Logging a session

<img src="walkthrough/07-session-view.png" width="320" alt="Session view — exercise accordion open, sets with weight and reps inputs">

Each exercise is an accordion. Open it, enter weight + reps for each set, tap ✓ to
log it. Inputs pre-fill with the values from the matching set in your last session,
so you can usually just tap-tap-tap through it. Stretching/balance exercises show a
duration timer. The session auto-saves to localStorage — close the browser
mid-workout, resume later.

Two exceptions to weight × reps:

- **Cardio** gets a **Z1–Z5 heart-rate zone grid** — one minutes box per zone, not a
  single min × RPE pair. Zones the coach prescribed show their `/ N min` target and
  start pre-filled at the planned value; the rest are blank. Minutes can be
  fractional, because Apple Health reports them that way. You never type an RPE for
  cardio: it's derived from the zone distribution you logged, so History, the
  per-exercise progress view, and the coach brief all read the same number. A
  *Paste cardio summary* box below the exercises turns an Apple Health
  workout-summary screenshot into those zone minutes (plus Claude's inferred RPE) —
  it appears for any cardio in the session, prescribed or ad-hoc.
- **Bodyweight exercises** drop the weight box and log reps only — unless they're one
  of the lifts people routinely load — pull-ups, chin-ups, dips, hanging leg raises,
  weighted crunches and a few others — which keep a weight input for the plate or belt
  you added. The library's `equipment` field tags all of those "body only", so the
  exceptions are a short curated allow-list in `src/library.rs`.

### 8. The heatmap closes the loop

<img src="walkthrough/08-heatmap-after.png" width="320" alt="History tab — body silhouette with worked muscles in green, plus session list below">

When you finish, you land on *History*. The body silhouette is colored by
**days since each muscle was last worked** — bright green for ≤3 days, fading
through paler greens to grey for "never." Below, the per-exercise session list.
Next time the coach plans for you, this heatmap is the recovery state it reasons
against — recently-hit muscles get prescribed less, neglected ones get prioritized.

### 9. Home knows you're done

<img src="walkthrough/09-home-done.png" width="320" alt="Workout tab after finishing — DONE badge replaces TODAY; CTA below points to generating tomorrow's workout">

Go back to *Workout* and the TODAY card has flipped to a **DONE** state — ✓ +
the workout name, "Today's workout is logged. Generate tomorrow's now, or wait
for the nightly coach run." No Start button. You can't accidentally re-enter
the same session, and the natural next action is the *Generate workout with
Claude* button right below it. (Tap it; you're back at step 4 for tomorrow.)

---

## Weird choices, and why

### 1. GitHub is the database

There is no server. There is no API. There is no Postgres, no Firebase, no Supabase, no Vercel function. Every byte of user data lives in:

1. `localStorage` in your browser (works offline; this is the source of truth).
2. Optionally, a single `state.json` file in a private GitHub repo you own.

That's it. To "sync across devices," you paste a fine-grained GitHub PAT into Options
and the app uses the [Contents API](https://docs.github.com/en/rest/repos/contents) to
read/write `state.json` with optimistic concurrency (the SHA-as-ETag trick). When the
app boots, it pulls the latest if the remote `updated_at` is newer than what it last
pushed. While you use it, every change schedules a 2-second debounced push back.

**Why this is actually a win:**
- **Zero infra cost.** GitHub already hosts the static app via Pages; the same GitHub
  hosts your data via a private repo. Bill goes from $0 → $0.
- **No backend to maintain.** Ever. Nothing to keep patched, nothing to scale, nothing
  to wake up to at 3am. The "server" is git.
- **You own your data as a JSON file in your own repo.** You can `git clone` it. You
  can pipe it through `jq`. You can write your own analyses. It's not locked in a
  proprietary store you can never get out of.
- **Cross-device sync without an account system.** No signup form, no password reset
  flow, no OAuth callbacks, no "forgot my account" emails. The PAT *is* the auth.
- **Audit trail comes free.** Every workout change is a git commit. `git log` is
  your history.

### 2. The AI plans the workout — and by default there's no AI *in* the app

This isn't an "AI-powered" wrapper around a chatbot. There's no API key for Anthropic
in the app, no Claude SDK call from the WASM bundle, no streaming response. The app
*generates a markdown brief*; you paste it into whichever Claude conversation you're
already using; you paste the JSON response back. The roundtrip happens entirely
outside the app.

Why this is the right call:
- **No API key to store.** A WASM app can't safely hold a long-lived Anthropic key.
  Pushing the roundtrip to your own Claude session means your existing auth handles
  it — and Anthropic pays for compute, not the app maintainer.
- **You can use whichever Claude surface you like.** Mobile app, claude.ai, Claude
  Code, an Opus session you're already deep into. They're all interchangeable.
- **The same flow handles Apple Health.** Want the coach to know your VO2 max?
  Paste an Apple Health screenshot into the same Claude conversation alongside the
  brief. Claude reads the value off the image, returns it as a top-level `vitals`
  block in the JSON response, and the app's importer updates your goals with it
  (dropping stale source-dates silently). The same trick handles cardio *actuals*:
  a workout-summary screenshot comes back as per-zone minutes plus an inferred
  1–10 RPE, which the session view writes straight onto the logged set. Re-import
  the same workout and it corrects the existing entry in place rather than adding
  a duplicate.
- **You can also run the same prompt unattended.** `scripts/coach/coach.sh` plus
  Cowork's `/schedule` gives you a nightly agent that pulls your state from GitHub,
  fetches the library, builds the brief, parses the response, validates every
  `library_id` against the bundled library, and pushes the new workout back to your
  data repo. It doesn't keep its own copy of the prompt: it shells out to the
  `coach-brief` binary, which calls the same `coach::build_coach_packet` the in-app
  Coach Brief does, so the nightly agent and the button in your pocket can't drift
  apart. You wake up, open the PWA, today's session is there.

**The opt-in exception.** Copy/paste is the default and always works, but it is a
chore three times over, so Options takes *your own* Anthropic key. Save one and the
same buttons call `api.anthropic.com` directly: *Generate with Claude* posts the very
same brief and runs the very same parser, and the cardio/VO2 imports grow a *Choose
screenshot* picker that sends the image as a base64 block with the prompt the copy
flow already uses. One model constant, an explicit reasoning effort, and a
`stop_reason` check that refuses to import a refusal or a truncated response.

The design holds even so: it's *your* key, not the app's; it lives in its own
`localStorage` entry on that one device and never enters `state.json`, the GitHub
sync or the CSV export; and nothing is removed — every copy/paste control stays
exactly where it was, which is what you fall back to when a call 401s or the network
drops.

### 3. Rust → WASM → PWA on iPhone Safari

The whole frontend is written in [Leptos](https://leptos.dev/) (a reactive Rust UI
framework), compiled to WebAssembly via [Trunk](https://trunkrs.dev/), and deployed
to GitHub Pages. The target is *iPhone Safari, opened from a Home Screen icon*.

Why Rust + WASM + PWA when basically every other production frontend is React?
- **Type safety end-to-end.** The same `ScheduledWorkout` struct serializes to
  localStorage, posts to GitHub, parses from a Claude response, and renders into
  the DOM. No DTO layer, no validation library, no zod schemas — `serde` and
  `Result<T, E>` carry the contract.
- **Mobile bundle size is fine.** The whole app gzips to ~300 KB.
- **It runs offline.** A service worker caches the app shell cache-first; the
  library JSON is network-first with HTTP-cache bypass. You can lose service in
  the gym basement and the PWA still loads and logs.
- **No app-store review.** "Add to Home Screen" installs it like a native app,
  with its own dumbbell icon, splash screen, and standalone window — but you ship
  updates by publishing a GitHub Release, no Apple/Google review cycle. (Merging to
  `main` is staging; a published release is what deploys to Pages, so the live app
  only moves when someone decides it should.)

**And the killer iOS-PWA quirk that justifies the no-account design:** Google
OAuth on iOS PWAs is *fundamentally broken*. The OAuth redirect bounces you to
Safari rather than back into your PWA window, breaking the auth round-trip — a
known issue Apple has [years of bug reports on](https://www.google.com/search?q=ios+pwa+google+oauth+broken)
and shows little urgency to fix. Building yet another login flow you'd have to
fight Safari every step of the way would have been miserable. Skipping accounts
entirely and using a GitHub PAT — which the user pastes once and never thinks
about again — sidesteps the whole pile.

---

## Other oddnesses worth knowing about

- **Per-set pre-fill from history.** When you start a workout, each set's weight
  and reps pre-populate from the matching `set_number` of your last completed
  session for that exercise (matched by `library_id`, falling back to exact name).
  Pyramid sets, drop sets, and warm-up sets all preserve their structure across
  sessions.
- **Log anything, any day, from the Exercises tab.** You don't need a planned
  session to log work. Search the library, tap an exercise, log sets straight onto
  an ad-hoc entry. A **"Logging for" date picker** at the top of the tab (capped at
  today, with a *⏱ Backdated* marker when it isn't today) backfills yesterday's
  session when you forgot in the moment — it sets both the entry's date and each
  set's completion date, so the heatmap and the coach brief see the work on the day
  it actually happened.
- **Two body silhouettes, two semantics.** The History heatmap colors by *days
  since strength-worked*. The Coach Brief separately tracks *days since last
  stretched* per muscle, which feeds into mobility prescription. Same body, two
  different views of recovery. Stretching and balance work never counts as
  "worked" on either the heatmap or the recovery table — a 30s cooldown stretch
  isn't a training stimulus, and crediting it told the coach a muscle was still
  recovering when it wasn't.
- **Library validation is enforced both client- and server-side.** The coach
  packet inlines every valid exercise ID; the importer rejects any
  `library_id` not in the bundled library; the nightly `coach.sh` aborts the
  push if any returned exercise isn't a real ID. Hallucinated lifts can't
  silently land in your plan.
- **Schema versioning, but only one version.** `state.json` has a
  `schema_version: 2`. Legacy v1 states (with a now-removed `plan` field) still
  deserialize cleanly. New optional fields are added with `#[serde(default)]` so
  rolling forward never breaks existing saved state.
- **CSV export.** The History tab has *Export CSV*, dumping every set ever logged
  (date, day, exercise, set #, reps, weight, completed, duration). If you ever
  outgrow the app, you walk away with a CSV.
- **Walkthrough-as-tests.** This page's screenshots come from
  `tests/playwright/walkthrough_hypertrophy.spec.ts` and `walkthrough_legday.spec.ts`
  — the same e2e tests that prove the goal → coach → session → history flow works
  end-to-end. Updating the screenshots is just re-running the harness
  (`npx playwright test --config=playwright.walkthrough.config.ts`) and copying the
  raw shots over the curated ones; the mapping is in the comment at the top of this
  file.

---

## Try it

Open <https://tbaums.github.io/wholesome-swolesome/> on your phone. On iOS Safari:
tap Share → Add to Home Screen for the standalone-app experience. No signup, no
password — start poking around immediately.

To wire up sync, follow the [README's sync section](../README.md#optional-cross-device-sync-github-backend).
For implementation details — module map, schema, coach loop — see [CLAUDE.md](../CLAUDE.md).

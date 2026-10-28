# Changelog

All notable changes to **wholesome-swolesome**. Versions track `.release-version`
(see `RELEASING.md`).

**Convention (matches the transom repo's strictness):** every entry carries a
**code** ref and a **docs** ref —
`- **Change** (#NN, code <sha>, docs <sha>) — …`. Use the same sha when the docs
rode in the implementing PR, distinct shas when docs landed separately, and
`docs none — internal` for changes with no user-facing surface. **The docs ref
is mandatory** — a change is not done until its docs are in. Author docs in the
build cycle, not at release time.

**Versioning (semver):** `MAJOR.MINOR.PATCH`. Bump **PATCH** for bug fixes with
no new user-facing capability, **MINOR** for new features/capabilities, **MAJOR**
for breaking changes. See `RELEASING.md`.

## [Unreleased]

### Added
- **Optional Claude API key — generate workouts and read screenshots in-app** (#59, code this change, docs this change) — every Claude step was a copy/paste round trip. Options now has a *Claude API key (optional)* card (Save / Test key / Clear); with a key saved, *Generate with Claude* posts the existing `build_coach_packet` brief to `api.anthropic.com` and schedules the result through the existing `parse_workout_response`, and the cardio-actuals import takes an Apple Health screenshot directly as a base64 image block with the prompt the copy flow already uses. Model `claude-opus-5-5` and an explicit `output_config.effort` live in one constant each; a `refusal` or `max_tokens` `stop_reason` imports nothing and says why; 401/429/network failures are plain toasts. The key lives in its own `ws_claude_api` localStorage entry — never inside `SyncConfig`/`SyncedState`, so it cannot reach `state.json` — with a Playwright test that decodes the sync PUT body to prove it. With no key saved every flow is byte-for-byte what it was.

### Documentation
- **README, coach README, `RELEASING.md` and `CLAUDE.md` brought back in line with the code** (#60, code none — docs only, docs this change) — audited line by line against the workflows, `src/components/options.rs`, `src/sync.rs`, `coach.sh` and `PROMPT.md`, correcting only what was actually wrong. `RELEASING.md` contradicted itself on what deploys a release — step 3 said publishing the Release fires `deploy.yml`, which it cannot, since events raised by `GITHUB_TOKEN` are suppressed — and its top section was itself stale on *where*: `release.yml` dispatches `deploy.yml` at `main`, not at the tag, because the `github-pages` environment only deploys `main` (#53). It now also states the branch flow: PRs land on `staging`, `main` fast-forwards from it, and a `.release-version` bump releases when it reaches `main`. The "Manually redeploy" and "Roll back" sections followed from that: a dispatch builds the branch it was dispatched from, and re-running an old deploy run rebuilds today's `main` rather than rolling anything back. `CLAUDE.md` still said Pages deploys on push to `main` (wrong since #44/#54/#55) and that releases are PR'd to `main`. `README.md`'s sync step 3 called the *Repo (owner/name)* field's pre-filled `you/wholesome-swolesome-data` a placeholder — it is a real value the user must replace — and named the token field and the connected/not-found statuses imprecisely. `scripts/coach/README.md` omitted `uuidgen` and the Rust toolchain from its prerequisites (`coach.sh` builds `coach-brief` when it is missing) and called `bash -x coach.sh` a dry run, which still pushes; its scratch directory is also removed by an `EXIT` trap rather than left behind. Verified-correct statements — the 8081 dev port, the 290-exercise library, the goals/cardio card fields, the 2-second push debounce, the shared-generator description and `PROMPT.md`'s Steps 3–4 fallback — were left untouched.
- **Walkthrough refreshed against everything merged since 2026-06-01** (#58, code none — docs only, docs this change) — `docs/walkthrough.md` had not moved since `ad76b42` while ~15 PRs (#24–#57) changed what it describes. Rewritten against the source: the full-screen note editor for injuries/coach notes (#42), the Z1–Z5 heart-rate zone grid and zone-derived RPE that replaced min × RPE for cardio (#28–#32, #51), the weight input kept for commonly-loaded body-only lifts (#27), the Exercises-tab "Logging for" date picker (#26), Apple Health re-import correcting in place (#39), the coach brief's planned-vs-logged / no-invented-preferences / stretching-excluded-from-recovery rules (#33–#35), the nightly agent running off the shared `coach-brief` generator (#38), and Pages deploying from published releases rather than every push to `main` (#43, #53). All nine curated screenshots recaptured. The walkthrough harness took `fullPage` captures with the app's chrome still fixed/sticky, so Chromium painted the bottom nav and the `.page-header` at their viewport position — across the middle of every image taller than the viewport, hiding the Sync *Pull/Push* buttons and the weekly-cardio input on the Options shot, among others. `fullPageShot` now neutralises that chrome for the capture (header `static`, nav parked at the document's true bottom) and `viewportShot` covers the two Coach Brief shots, which are one document at two scroll positions and would otherwise collapse into the same image. The curated→spec→raw mapping and the capture modes are recorded at the top of the walkthrough, and `walkthrough_hypertrophy.spec.ts` drives the note editor instead of the removed inline textareas.

### Fixed
- **Auto-deploy dispatches `main`, not the release tag** (#53, code this change, docs none — internal) — the `github-pages` environment's deployment-branch policy only permits `main`, so dispatching `deploy.yml` at the tag was rejected (`BlobNotFound`); v0.5.1's first auto-deploy attempt hit this. `release.yml` now dispatches `main`, which is the just-tagged commit, so the release still deploys exactly its own code. Rolls into the next release.

## [0.5.1] — 2026-07-21

### Fixed
- **Releases now auto-deploy to GitHub Pages** (#53, code this release, docs this release) — `release.yml` now dispatches `deploy.yml` at the freshly-created tag. The `release: published` event never triggered `deploy.yml` because events raised by `GITHUB_TOKEN` are suppressed to prevent workflow recursion, so every release (v0.4.0, v0.5.0) sat undeployed until a manual dispatch. `workflow_dispatch` is a documented exception `GITHUB_TOKEN` may trigger, so the chain now completes end-to-end.

### Changed
- **Release + docs conventions codified** (code this release, docs this release) — `RELEASING.md` now specifies semver (patch for bug fixes, minor for features) and this `CHANGELOG.md` is now mandatory per change, matching the transom repo's documentation strictness (per-item code + docs refs, docs authored in the build cycle).

## [0.5.0] — 2026-07-21

### Fixed
- **Cardio RPE is derived from the logged HR zones** (#51, code df4aeb4, docs 0.5.1 backfill) — zone-logged cardio (e.g. elliptical) displayed a frozen RPE regardless of the minutes logged per zone (an all-Zone-1 workout still read the same high value). The logged zones are now authoritative: a minute-weighted average of per-zone anchors (matching the app's documented zone→RPE mapping), applied at every read site (History, Progress, Coach brief, CSV). Existing history is corrected retroactively, since zone minutes are stored per set.

---

_Entries before 0.5.1 were backfilled when this changelog was introduced (it did
not exist during the 0.1.0–0.5.0 releases)._

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

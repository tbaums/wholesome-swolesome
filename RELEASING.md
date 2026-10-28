# Releasing

**Production = the latest published GitHub Release.** The live GitHub Pages site
is built and deployed when a release is cut — *not* from every commit that lands
on `staging` or `main`.

- Work lands on **`staging`**: PRs are opened against `staging`, and `ci.yml`
  gates every PR. `main` then **fast-forwards from `staging`**. Neither merging
  to `staging` nor fast-forwarding `main` touches the live site.
- Cutting a **release** is what promotes to **production**: `release.yml` cuts
  the tag + GitHub Release, then **dispatches `deploy.yml` at `main`** — the
  commit it just tagged — which runs `trunk build --release` and deploys `dist/`
  to Pages.

> **Why the explicit dispatch (not the `release: published` event):** a release
> created by `release.yml` uses the default `GITHUB_TOKEN`, and GitHub suppresses
> events raised by `GITHUB_TOKEN` from triggering further workflows (anti-
> recursion). So the `release: published` event never starts `deploy.yml`.
> `workflow_dispatch` is one of the two documented exceptions `GITHUB_TOKEN` may
> trigger, so `release.yml` ends by running `gh workflow run deploy.yml --ref
> main`. (Before this was wired up, v0.4.0 and v0.5.0 were tagged but never
> deployed until dispatched by hand — see #53.)

> **Why `--ref main` and not the tag:** the `github-pages` environment's
> deployment-branch policy only permits `main`, so a run on a tag ref is rejected
> at the deploy step (it surfaces as `BlobNotFound`) — v0.5.1's first auto-deploy
> hit exactly that. Dispatching `main` builds the same code anyway, because
> `release.yml` tagged `main`'s tip.

## Versioning

Tags are semver `vMAJOR.MINOR.PATCH`:

- **PATCH** (`v0.5.0` → `v0.5.1`) — bug fixes with no new user-facing capability.
- **MINOR** (`v0.5.1` → `v0.6.0`) — new features / user-facing capabilities.
- **MAJOR** — breaking changes.

Pick the bump from what actually changed, not by habit. (The 0.1.0–0.4.0 releases
predate this rule and were all minor bumps; from 0.5.1 on, follow semver.)

## Documentation (required, not optional)

Every change that ships carries its documentation **in the same PR** — docs are
authored in the build cycle, not bolted on at release time:

- **`CHANGELOG.md` is mandatory.** Add an entry under the target version with the
  house format `- **Change** (#NN, code <sha>, docs <sha>) — …`. Same sha when the
  docs rode in the implementing commit; distinct when docs landed separately;
  `docs none — internal` for changes with no user-facing surface. The **docs ref
  is mandatory** — a change is not "done" until its docs are in.
- Update `README.md` / `docs/` whenever behavior a user relies on changes.
- A release must not introduce a user-facing change that has no CHANGELOG entry.

This matches the transom repo's documentation strictness.

## Cut a release (promote main to production)

The version-file path is the normal flow (and the only one remote agents can use,
since they can't push tags or dispatch workflows):

1. Bump `.release-version` to the new tag (e.g. `v0.1.0` → `v0.2.0`) on a branch
   and open a PR into `staging` — version bumps go through `staging` like every
   other change.
2. Merge the PR, then fast-forward `main` from `staging`. `release.yml` watches
   `.release-version` on **`main`**, so it fires when the bump arrives there,
   creates the tag at that commit, and publishes a GitHub Release with
   auto-generated notes.
3. `release.yml` then dispatches `deploy.yml` itself (at `main`, which is the
   commit it just tagged), and that builds and deploys to Pages. The live site
   now reflects the new release.

Re-running `release.yml` for a tag that already exists is a no-op, so a no-change
merge to `.release-version` can't clobber a published release.

> Alternative: maintainers with push access can instead run **Actions → Release →
> Run workflow** from `main` and pass the tag directly — it tags `main`'s tip, so
> same downstream effect.

## Manually redeploy the current release

Use this if a deploy failed or Pages needs a rebuild without cutting a new tag:

- **Actions → Deploy to GitHub Pages → Run workflow** (`workflow_dispatch`),
  **from `main`**.
- On a manual run there is no release tag, so the build checks out the branch the
  run was dispatched from and deploys its tip. Dispatch from `main`: it is the
  only branch the `github-pages` environment will deploy. To put a *specific*
  older release back up, use the rollback flow below.

## Roll back to a prior release

Pages only ever deploys `main`, so a rollback means getting the older code onto
`main` and redeploying it:

- Revert the offending change on `staging` and fast-forward `main`, then
  **Actions → Deploy to GitHub Pages → Run workflow** from `main`. Cutting a new
  PATCH release off that reverted `main` does the same thing and keeps the
  release history honest about what is live.
- Re-running an *old* deploy run does **not** roll back: releases are deployed by
  a dispatch at `main`, so re-running one rebuilds whatever `main` points at now.

## Service-worker cache (`public/sw.js`)

`public/sw.js` holds a `CACHE` constant (e.g. `swolesome-v16`). It is **cache-first
for the app shell**, so returning clients keep serving the old build until the
constant changes.

**Whenever a release changes shipped assets (WASM/JS/CSS/HTML), bump `CACHE`**
(e.g. `swolesome-v16` → `swolesome-v17`) in the same change so clients drop the
stale shell and pick up the new build on next load. A release that ships no asset
change (e.g. a CI/workflow-only change) does not need a bump.

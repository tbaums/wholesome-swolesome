# Releasing

**Production = the latest published GitHub Release.** The live GitHub Pages site
is built and deployed from the release's tag — *not* from every commit on `main`.

- `main` is **staging**: CI (`ci.yml`) gates it, but merging to `main` no longer
  touches the live site.
- Publishing a **release** is what promotes to **production**: `release.yml` cuts
  the tag + GitHub Release, and `deploy.yml` (triggered on `release: published`)
  checks out that exact tag, runs `trunk build --release`, and deploys `dist/` to
  Pages.

## Cut a release (promote main to production)

The version-file path is the normal flow (and the only one remote agents can use,
since they can't push tags or dispatch workflows):

1. Bump `.release-version` to the new tag (e.g. `v0.1.0` → `v0.2.0`) on a branch
   and open a PR into `main`.
2. Merge the PR. `release.yml` fires on the `.release-version` change, creates the
   tag at the merge commit, and publishes a GitHub Release with auto-generated
   notes.
3. Publishing that Release fires `deploy.yml`, which builds the tagged commit and
   deploys it to Pages. The live site now reflects the new release.

Re-running `release.yml` for a tag that already exists is a no-op, so a no-change
merge to `.release-version` can't clobber a published release.

> Alternative: maintainers with push access can instead run **Actions → Release →
> Run workflow** and pass the tag directly. Same downstream effect.

## Manually redeploy the current release

Use this if a deploy failed or Pages needs a rebuild without cutting a new tag:

- **Actions → Deploy to GitHub Pages → Run workflow** (`workflow_dispatch`).
- On a manual run there is no release tag, so the build checks out the default
  branch (`main`) and deploys the tip of main. To redeploy a *specific* release
  instead, use the rollback flow below.

## Roll back to a prior release

To put an older release back on the live site, re-run that release's deploy:

- **Actions → Deploy to GitHub Pages**, open the run from when that release was
  published, and **Re-run all jobs**. It re-checks-out that tag and redeploys it.
- If no such run exists, re-publish the target Release from the Releases page
  (unpublish → publish, or edit + publish); publishing fires `deploy.yml` for that
  tag again.

## Service-worker cache (`public/sw.js`)

`public/sw.js` holds a `CACHE` constant (e.g. `swolesome-v16`). It is **cache-first
for the app shell**, so returning clients keep serving the old build until the
constant changes.

**Whenever a release changes shipped assets (WASM/JS/CSS/HTML), bump `CACHE`**
(e.g. `swolesome-v16` → `swolesome-v17`) in the same change so clients drop the
stale shell and pick up the new build on next load. A release that ships no asset
change (e.g. a CI/workflow-only change) does not need a bump.

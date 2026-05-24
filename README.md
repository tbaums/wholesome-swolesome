# Wholesome Swolesome 💪

A mobile-first PWA workout tracker built in Rust + Leptos (WASM). Instead of hand-editing a static plan, you set training goals once and **Claude generates each day's workout** based on your goals, recent training history, and per-muscle recovery state.

**Live app:** https://tbaums.github.io/wholesome-swolesome/

## What's in the app

- **Today** — the workout the coach has planned for today (generated locally, or imported from a nightly agent run)
- **Library** — 133 exercises from [free-exercise-db](https://github.com/yuhonas/free-exercise-db) with photos, primary/secondary muscles, and a body silhouette per detail page
- **Exercises** — freeform logging for anything not in a planned workout
- **History** — a body heatmap colored by days-since-last-worked (≤3d / 4–7d / 8–14d / 15+) over a list of past sessions
- **Options** — set your training goals (primary focus, sessions/week, session minutes, equipment, injuries/notes) and configure optional GitHub sync

All data lives in `localStorage`. There is no account and no backend. Optional GitHub sync persists state to a private repo so you can use the app across devices.

## Using the app, end-to-end

1. **Set your goals.** Open *Options* → fill in the *Training goals* card. Primary goal (hypertrophy / strength / fat loss / endurance / general), sessions per week, session minutes, available equipment, anything to avoid, freeform notes. These are what the coach plans against.

2. **Get tomorrow's workout.** There are two paths, designed to be interchangeable:

   - **In-app, on demand.** On the Home tab tap *🧠 Generate workout with Claude*. The *Coach Brief* view renders a markdown packet (your goals + recent training + per-muscle recovery state + the library reference). Copy it, paste it into any Claude Code chat, copy Claude's JSON response back, paste it into the bottom textarea, and tap *Import workout*. The workout lands in your scheduled list for the date you picked.
   - **Nightly, unattended.** Run `scripts/coach/coach.sh` once to verify it works, then schedule it via Cowork to run every night. See [scripts/coach/README.md](scripts/coach/README.md) for the full setup, including the `/schedule` invocation. The nightly agent does the same flow as the in-app button, but writes the result back to your sync repo so the app picks it up next time it loads.

3. **Run the workout.** On the Home tab, the *TODAY* card shows the scheduled workout. Tap *Start workout →* to enter the session view. Open each exercise's accordion, enter weight + reps for each set, tap ✓ to log it. The session auto-saves; you can close the browser mid-workout and resume.

4. **Finish.** Tap *Finish Workout*. The session becomes a permanent history entry. The History tab updates the heatmap — the muscles you just worked turn deep green.

5. **Repeat.** The coach reads the new history on its next run and adjusts: recently-hit muscles get prescribed less, neglected ones get prioritized, progressive overload is applied within rep ranges.

## Optional: cross-device sync (GitHub backend)

The app can persist state to a private GitHub repo so you can use it from both phone and laptop without re-entering data.

1. Create a private repo, e.g. `you/wholesome-swolesome-data`, with an empty `state.json`.
2. In the app, *Options → Sync (GitHub)*, paste a fine-grained personal-access token with `Contents: read+write` on that repo.
3. Tap *Test connection*, then *Push to GitHub*.
4. On any other device, sign in to the app, paste the same token + repo, tap *Pull from GitHub*.

After that, the app debounces a push 2 seconds after any data change, and pulls fresh state on boot when the remote is newer than the local last-push timestamp.

## Development

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve              # dev server at http://localhost:8080
trunk build --release    # production build into ./dist
```

For deploying to GitHub Pages (sets the correct base URL):

```bash
trunk build --release --public-url /wholesome-swolesome/
```

### Dev container

A devcontainer with the full toolchain (Rust + wasm32, Trunk, wasm-pack, Node, Playwright Chromium, Claude CLI, gh) is in [.devcontainer/](.devcontainer/). Run `./.devcontainer/run.sh` to bring it up and drop into a shell.

### Tests

```bash
# Rust unit tests in WASM (state hydration, schema versions, library parsing)
wasm-pack test --headless --firefox --lib

# Playwright E2E (matches CI)
npx playwright test

# Playwright E2E locally in dev container (Chromium swap for missing WebKit)
npx playwright test --config=playwright.local-chromium.config.ts

# Walkthrough harness — full end-to-end flows with screenshots
npx playwright test --config=playwright.walkthrough.config.ts
```

Screenshots from walkthrough runs land in `tests/playwright/screenshots/<spec-name>/` (gitignored).

## Architecture pointer

For implementation details — module map, state model, schema versions, etc. — see [CLAUDE.md](CLAUDE.md).

## License

MIT — see [LICENSE](LICENSE).

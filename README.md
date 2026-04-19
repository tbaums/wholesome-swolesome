# Wholesome Swolesome 💪

A mobile-first PWA workout tracker built in Rust + Leptos (WASM). Track sets, reps, and weight while you're in the gym. No account, no backend — all data lives in your browser's localStorage.

**Live app:** https://tbaums.github.io/wholesome-swolesome/

## Features

- Log sets, reps, and weight for each exercise
- Weights auto-fill from your last session for that day
- Session progress saved automatically — survives closing the browser
- Full workout history with per-session detail view
- Progress charts per exercise
- Editable workout plan with CSV import/export
- Installable as a PWA (Add to Home Screen on iPhone)

## Tech stack

- [Leptos](https://leptos.dev) 0.7 (CSR) compiled to WASM via [Trunk](https://trunkrs.dev)
- `web-sys` for localStorage, Blob/URL for CSV download
- `serde_json` for serialization
- Deployed to GitHub Pages via GitHub Actions

## Development

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve          # dev server at http://localhost:8080
trunk build --release --public-url /wholesome-swolesome/  # production build
```

## License

MIT — see [LICENSE](LICENSE).

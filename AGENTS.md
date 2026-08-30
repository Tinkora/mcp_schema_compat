# Agent instructions

- Use English Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`, `ci:`).
- Write new code comments in English and explain intent, not syntax.
- Before a Rust change, run `cargo fmt --all -- --check`, `cargo test --locked`, and `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- Keep changes focused, offline-safe, deterministic, and covered by outcome-focused tests.
- Default user documentation to English and keep `README.zh-CN.md` and Chinese governance documents in sync.

# Rust add-on

Language conventions layered on top of `.agent/STANDARDS.md`. Pulled in when the
repo contains Rust.

## Build & checks
- The done bar for Rust: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`. All three green before you claim done.
- Treat clippy warnings as errors. Do not `#[allow(...)]` to silence a lint without a one-line comment saying why.

## Idioms
- Return `Result<T, E>` with a typed error (`thiserror` for libraries, `anyhow` for binaries). Never `unwrap()`/`expect()` on a fallible path that can be hit at runtime; reserve them for invariants that genuinely cannot fail and say why.
- Prefer borrowing over cloning; reach for `Arc`/`clone` only when ownership actually needs to be shared. Don't scatter `.clone()` to dodge the borrow checker — restructure.
- Model illegal states out of existence with enums and newtypes instead of validating primitives everywhere.
- Keep `unsafe` out unless there is no safe alternative; when unavoidable, isolate it, document the invariant it upholds, and test it hard.

## Async
- One runtime (`tokio`). Don't block the executor — no `std::fs`/`std::thread::sleep` in async fns; use the async equivalents or `spawn_blocking`.
- Don't hold a `std::sync::Mutex` guard across an `.await`. Use `tokio::sync` primitives for contended async state.

## Dependencies
- Justify every new crate (maintenance, license, transitive weight). Prefer the workspace's existing choices over a second crate that does the same job.

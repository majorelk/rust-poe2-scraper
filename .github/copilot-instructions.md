# GitHub Copilot Instructions — rust-poe2-scraper

## What this repo is
An async Rust CLI that scrapes PoE2 trade data, stores it in SQLite, and computes basic, bias-aware normalisation of modifiers across item bases. Scope is CLI + persistence + normalisation; no UI.

## How to build, test, lint
- Format: `cargo fmt`
- Lint: `cargo clippy -- -D warnings`
- Test: `cargo test`
- Run: `cargo run -- <subcommand> [flags]`
- If DB migrations exist: `sqlx migrate run` (or keep `rusqlite` if SQLX is not used).

The agent must run all three before opening a PR: **fmt**, **clippy -D warnings**, **test**.

## Runtime config (env)
Provide sensible defaults. Never hard-code secrets.
- `DB_URL` (default `sqlite://poe_items.db`)
- `TRADE_BASE_URL`
- `USER_AGENT` (identify politely)
- `REQUESTS_PER_MIN` (default `30`)
Load via `dotenvy` or `envy`. Log the effective config at startup without secrets.

## Libraries we expect
- HTTP: `reqwest`, `tokio`
- Rate-limit/retry: `governor` or `tower` + backoff (respect `Retry-After`)
- CLI: `clap`
- Serde: `serde`, `serde_json`, optionally `scraper` if HTML
- DB: `sqlx` with `sqlite` feature **or** `rusqlite` (choose one and stay consistent)
- Logging: `tracing`, `tracing-subscriber` (text + JSON mode)
- Metrics (nice to have): `metrics`, `metrics-exporter-prometheus`

## Coding standards
- No `.unwrap()`/`.expect()` in non-test code. Use `anyhow` or `thiserror` with context.
- Small diffs (aim ≤ ~50 lines) with matching tests.
- Idempotent writes to DB (UPSERT/UNIQUE constraints). Avoid duplicates.
- Respect target sites: add rate-limit, polite headers, backoff, and a `--dry-run`.
- Keep modules cohesive: `config.rs`, `telemetry.rs`, `net.rs`, `model.rs`, `db.rs`, `scrape.rs`, `normalize.rs`, `main.rs`.

## Branching & commits
- Create feature branches: `feat/<short-task>`.
- Semantic commits: `feat: …`, `fix: …`, `chore: …`, `docs: …`, `test: …`.
- Each PR must include: what/why, how to run, and acceptance checks (below).

## Definition of done (every task/PR)
- [ ] Code, tests, docs updated
- [ ] `cargo fmt` and `cargo clippy -- -D warnings` pass
- [ ] `cargo test` green
- [ ] If DB touched, a migration + test that verifies idempotency
- [ ] Logs show useful telemetry (counts, durations, errors by kind)

## Safe-ops / boundaries for the agent
- Do **not** remove licenses, changelogs, or security files.
- Do **not** push large fixtures or scraped datasets—use small samples under `tests/fixtures/`.
- If live endpoint is flaky, default to fixture-based tests and mark real integration tests `#[ignore]`.
- Keep network concurrency and RPS conservative by default; make them configurable.

## Initial task queue for the agent (execute in order)
1) **Bootstrap**  
   - Add `.env.example` with `DB_URL=sqlite://poe_items.db`, `TRADE_BASE_URL=…`, `USER_AGENT=…`, `REQUESTS_PER_MIN=30`.  
   - Add Makefile (or `justfile`) targets: `fmt`, `lint`, `test`, `run`.  
   - Update `README.md` with "Dev quickstart".  
   **Accept:** build/test pass; quickstart works.

2) **Config + Telemetry**  
   - `src/config.rs` loads env with defaults; `src/telemetry.rs` sets `tracing_subscriber` (env-switchable JSON).  
   - Main logs config (redact secrets).  
   **Accept:** unit tests for defaulting; visible startup log.

3) **HTTP client + rate limit + retry**  
   - `src/net.rs` wraps `reqwest`; set `USER_AGENT`, gzip, exponential backoff w/ jitter; respect 429/5xx + `Retry-After`.  
   **Accept:** unit tests for backoff math + header handling; `#[ignore]` integration test against httpbin.

4) **Models**  
   - `src/model.rs` with `Listing`, `Item`, `Modifier`, `ScrapeMeta` (`serde` derive; tolerate partials).  
   **Accept:** roundtrip tests using `tests/fixtures/`.

5) **Persistence**  
   - Migrations for `listings`, `items`, `modifiers`, `listing_mods`, `scrape_runs` with UNIQUE/indexes.  
   - `src/db.rs` with `insert_or_ignore_*` + `begin_scrape_run/end_scrape_run`.  
   **Accept:** sqlx test shows duplicate insert ignored.

6) **Scrape subcommand**  
   - `src/scrape.rs` + `clap` subcommand `scrape --query @examples/sample.json` (or env).  
   - Parse → map → persist; log counts/durations/errors.  
   **Accept:** DB filled; stats logged; parser test on canned sample.

7) **Normalise subcommand**  
   - `src/normalize.rs` + CLI `normalize` that bins modifiers across bases and writes `normalized_mod_stats`.  
   **Accept:** unit test on synthetic data; CLI prints histogram summary.

8) **Metrics endpoint (optional)**  
   - `--metrics 0.0.0.0:9464` serving Prometheus metrics.  
   **Accept:** manual curl shows non-zero counters.

> The agent should proceed task-by-task and open a **separate PR per task** with a clear checklist and passing CI.

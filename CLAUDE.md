# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

Use `nix develop` to enter the development shell before running most commands. The Nix shell provides InfluxDB, Telegraf, Nomad, Holochain, and other tooling.

For one-off commands that need these tools, use `nix develop -c <command>` to run a single command in the Nix environment without entering an interactive shell.

```bash
# Build default workspace members (framework, bindings, summariser, happ_builder)
cargo build

# Run all tests
cargo test --workspace --all-targets

# Run tests for a single crate
cargo test -p holochain_summariser

# Lint (must pass with no warnings)
cargo clippy --workspace --all-targets --all-features -- --deny warnings

# Format Rust
cargo fmt --all

# Format TOML
taplo format

# All static checks (see scripts/checks.sh for full list)
nix develop -c bash -c "source scripts/checks.sh && check_all"
```

Scenarios and zomes are **excluded from the default workspace members** and must be built explicitly:

```bash
# Run a scenario locally
RUST_LOG=info cargo run -p zome_call_single_value -- --duration 60

# Smoke-test a scenario through Nix (as CI does)
nix run .#rust-smoke-test -- --package zome_call_single_value -- --duration 5 --no-progress
```

## Project Architecture

### Framework (`framework/`)

Generic load-testing infrastructure with no Holochain-specific code:

- `wind_tunnel_core` — shared types and traits
- `wind_tunnel_instruments` / `wind_tunnel_instruments_derive` — metrics collection and procedural macros
- `wind_tunnel_runner` — scenario execution engine (agents, hooks, CLI)
- `wind_tunnel_summary_model` — `RunSummary` and related serializable types read from `run_summary.jsonl`

### Bindings (`bindings/`)

Adapt the framework to specific systems:

- `holochain_client_instrumented` / `holochain_wind_tunnel_runner` — Holochain bindings
- `kitsune_client_instrumented` / `kitsune_wind_tunnel_runner` — Kitsune bindings

### Scenarios (`scenarios/`)

Each scenario is a standalone binary using `holochain_wind_tunnel_runner` (or `kitsune_wind_tunnel_runner`). A `ScenarioDefinitionBuilder` wires up global setup/teardown hooks, per-agent setup/teardown hooks, and one or more agent behaviour functions. Scenarios are not in `default-members` and are built on demand.

Scenarios that require custom zomes reference a `build = "../scenario_build.rs"` build script and declare `[package.metadata.required-dna]` / `[package.metadata.required-happ]` / `[package.metadata.fetch-required-happ]` sections in their `Cargo.toml` to build zomes and package them into hApps at build time.

Common functionality available for scenarios:
- Scenarios which use an instrumented client like `holochain_client_instrumented` will automatically record metrics for client calls. Custom metrics can also be recorded by getting a `Reporter` from the scenario context.
- Setup/teardown hooks can be used to perform common tasks before or after the scenario. Use agent setup/teardown hooks for tasks that only apply to the current agent.
- The scenario can check whether the framework is trying to shut down to break out of retry loops or stop other long-running work.
- Named behaviors allow a scenario to be comprised of multiple agents, behaving differently while interacting with each other.
- Report which environment variables affect scenario behavior. This *must* be used for any variable that changes the scenario's behavior, otherwise the summariser can't recognize different configurations of the same scenario.

### Shared Scenario Libraries (`scenarios_common/`)

Reusable library crates shared across multiple scenario binaries. Each subdirectory is a Rust library (not a standalone binary) providing common helpers for a family of related scenarios.

- `unyt_scenario` (`wind_tunnel_unyt_scenario`) — shared infrastructure for the Unyt scenarios (`unyt_chain_transaction`, `unyt_chain_transaction_zero_arc`), including network initialization, agent setup, durable object communication, and behaviour logic.

### Zomes (`zomes/`)

Holochain coordinator/integrity zome pairs. Each zome uses `build = "../../wasm_build.rs"`. Coordinator and integrity zomes are separate Rust projects.

### Summariser (`summariser/`)

A utility tool (`holochain-summariser`) that queries InfluxDB for a completed run, produces structured JSON summaries, and writes a report file.

**Data flow:**
1. Reads `run_summary.jsonl` (or `$RUN_SUMMARY_PATH`) to find runs — `RunSummary` contains the key fields: `run_id`, `scenario_name`, `started_at`, `run_duration`, `fingerprint`.
2. Dispatches to a per-scenario `summarize_*` function (registered in `lib.rs`) which queries InfluxDB.
3. InfluxDB responses are converted to Polars `DataFrame`s via `frame.rs`.
4. Analysis functions in `analyze.rs` compute statistics.
5. Results are combined into `SummaryOutput` and serialised as JSON.

### Test Data for Summariser

The summariser has two Cargo features to enable snapshot testing:

- `test_data` — captures real InfluxDB query results to `summariser/test_data/2_query_results/` (keyed by SHA3-256 hash of the query string) and run summaries to `1_run_summaries/`. Enable when capturing new test data against a live InfluxDB.
- `query_test_data` — replays captured data in place of real InfluxDB calls. Unit tests automatically use this feature via `dev-dependencies`.

When the output is changed, the tests will print a diff of the new output against the corresponding snapshot from `summariser/test_data/3_summary_outputs/`. If the change is expected, run the tests again with `UPDATE_SNAPSHOTS=1 cargo test --test snapshot` to accept the new snapshots

**Changing queries invalidates snapshot test data** because the file name is the SHA3-256 hash of the query string. Plan query changes carefully; renaming existing query files is preferred over modifying them. It is acceptable to write temporary code to determine the old and new hashes to enable this renaming. Such temporary code must be cleaned up once the migration is done.

### Summary Visualiser (`summary-visualiser/`)

Go templates and shell scripts — not a Rust crate. Renders the JSON summary outputs as HTML for the GitHub Pages site.

## Summariser Design Constraints

- Assume a mixed estate of hardware. For example, computing the mean of remaining memory is not meaningful because different machines can have different quantities of RAM installed.
- The project uses Polars data frames for working with data. 
- Converting from Polars into other data structures like `HashMap` or `Vec` in Rust is sometimes necessary but should be avoided when possible. Using vectorised operations with Polars is always preferred for performance.
- If a vectorised operation is possible but limits the meaning of the output, that operation should be preferred but it *must* be communicated to the user.
- Limit the output size of summaries. For example, don't partition data by agent and report individual agent performance, but instead report overall performance across all agents.
- Key, actionable insights are preferred over having a large number of output summary statistics.
- Prefer fetching fields and tags from InfluxDB in a smaller number of queries where possible. Once converted to a Polars data frame, the data can be filtered and manipulated as required.
- A user-friendly interpretation of each output metric must be included on struct fields.
- Where interpreting the meaning of summary statistics requires some care, this must also be documented. For example, the `mean_time_above_80_percent_s` field which is *only* calculated for hosts that spent some amount of time above 80% usage.
- Common functions for computing summary statistics from data frames should have their behavior and re-usability clearly documented so that they can be appropriately applied.
- Backwards compatibility is not required to be maintained. If the source data changes, or a change is planned to the output summary statistics, then make the change directly without compatibility in mind.
- Changes to queries that result in empty values in summary outputs for test data are not acceptable because the tests aren't exercising the code properly in that case. The user must be informed to re-generate test data when this happens.

## PR Review Guidelines

When reviewing pull requests:

- **Be terse.** Only flag genuine issues: bugs, logic errors, security problems, or violations of project conventions documented here.
- **Do NOT comment on:** style preferences, minor naming choices, things that are already fine, or anything that `cargo clippy` / `cargo fmt` / `taplo format` would catch (CI handles those).
- **Do NOT leave praise or filler** like "nice work" or "looks good overall." If there are no issues, say "No issues found." and nothing else.
- **Each comment should be 1–3 sentences.** State the problem, why it matters, and include a short code snippet showing the suggested fix.
- **Focus on the diff.** Don't review unchanged code unless a change introduces a problem in surrounding context.
- **Understand the architecture** before commenting. Read the relevant sections above (Framework, Bindings, Scenarios, Summariser) so your feedback is informed by how the project actually works.

## Code Hygiene

- Generated Rust and TOML must always be properly formatted, if you're not sure then run `cargo fmt` or `taplo format` on the relevant files.

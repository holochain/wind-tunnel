# New Scenario Checklist

Use this checklist when a PR adds a new scenario to wind-tunnel. Copy the relevant items into the PR description and tick them off as the work lands.

## Core implementation

- [ ] Scenario binary created in `scenarios/<scenario_name>/` with `Cargo.toml` and `src/main.rs`
- [ ] Scenario added to workspace `members` in root `Cargo.toml`
- [ ] Scenario `Cargo.toml` has `[package.metadata]` present (even if empty, required for the hApp builder)
- [ ] If the scenario uses Holochain hApps/DNAs: `Cargo.toml` declares one of `[package.metadata.required-dna]`, `[package.metadata.required-happ]`, or `[package.metadata.fetch-required-happ]`
- [ ] `Cargo.toml` sets `build = "../scenario_build.rs"`
- [ ] Scenario code reports all behavior-changing environment variables via `ScenarioDefinitionBuilder::add_capture_env`
- [ ] Scenario README added with: purpose, env variables, metrics recorded, and instructions to run locally

## CI / Smoke tests

- [ ] Smoke test added to `.github/workflows/test.yaml`

## Nomad deployment

- [ ] Scenario vars JSON added to `nomad/job-variants/canonical/vars/<scenario_name>.json`
- [ ] Scenario vars JSON added to `nomad/job-variants/demo/vars/<scenario_name>.json`
- [ ] Scenario vars JSON added to `nomad/job-variants/canonical-scaled/vars/<scenario_name>.json` (only if scenario should run in the scaled estate)
- [ ] Scenario added to `job-name` matrix in `.github/workflows/nomad-canonical.yaml`
- [ ] Scenario added to `job-name` matrix in `.github/workflows/nomad-demo.yaml`
- [ ] Scenario added to `job-name` matrix in `.github/workflows/nomad-canonical-scaled.yaml` (only if applicable)

## Summariser (optional — implement or link tracking issue)

- [ ] Summariser scenario module added in `summariser/src/scenario/<scenario_name>.rs`
- [ ] Module registered in `summariser/src/scenario.rs` and dispatched in `summariser/src/lib.rs`
- [ ] Test data captured (run summaries, query results, snapshot outputs under `summariser/test_data/`)
- [ ] Capture entry added to `summariser/capture_all.sh`

## Summary visualiser (optional — implement or link tracking issue)

- [ ] Template added to `summary-visualiser/templates/scenarios/<scenario_name>.html.tmpl`
- [ ] Smoke test line added to `summary-visualiser/test.sh`

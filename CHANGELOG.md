# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## \[[0.5.0-dev.0](https://github.com/holochain/wind-tunnel/compare/v0.4.0-dev.1...v0.5.0-dev.0)\] - 2025-07-16

### Added

- Add job to comment the changelog preview on PRs by @cdunster in [#221](https://github.com/holochain/wind-tunnel/pull/221)
- Add missing CACHIX_AUTH_TOKEN env to cachix push step by @cdunster in [#219](https://github.com/holochain/wind-tunnel/pull/219)
- Add holochain_serialized_bytes dependency by @cdunster in [#213](https://github.com/holochain/wind-tunnel/pull/213)

### Changed

- Markdown format the CHANGELOG.md
- Bump holochain/actions from 1.0.0 to 1.2.0 by @dependabot[bot] in [#212](https://github.com/holochain/wind-tunnel/pull/212)
- Update PR template for conventional commits usage by @cdunster
- Update Cargo.lock file by @ThetaSinner
- Update flake.lock file by @ThetaSinner in [#216](https://github.com/holochain/wind-tunnel/pull/216)
- Update Cargo.lock file by @ThetaSinner in [#203](https://github.com/holochain/wind-tunnel/pull/203)
- Update flake.lock file by @ThetaSinner in [#201](https://github.com/holochain/wind-tunnel/pull/201)
- Bump AdityaGarg8/remove-unwanted-software from 2 to 5 by @dependabot[bot] in [#195](https://github.com/holochain/wind-tunnel/pull/195)
- Bump peter-evans/create-pull-request from 6 to 7 (#194) by @dependabot[bot] in [#194](https://github.com/holochain/wind-tunnel/pull/194)
- Use workspace package properties (#198) by @ThetaSinner in [#198](https://github.com/holochain/wind-tunnel/pull/198)
- Add release support (#193) by @ThetaSinner in [#193](https://github.com/holochain/wind-tunnel/pull/193)
- Maintenance update versions (#192) by @ThetaSinner in [#192](https://github.com/holochain/wind-tunnel/pull/192)
- Update flake.lock file (#191) by @github-actions[bot] in [#191](https://github.com/holochain/wind-tunnel/pull/191)
- Enable scenarios remote_call_rate, remote_signals & two_party_countersigning on nomad cluster by @jost-s in [#188](https://github.com/holochain/wind-tunnel/pull/188)
- Track and reduce disk usage (#189) by @ThetaSinner in [#189](https://github.com/holochain/wind-tunnel/pull/189)
- Update to use holochain  0.5 (#182) by @zippy in [#182](https://github.com/holochain/wind-tunnel/pull/182)
- Use less disk space (#185) by @ThetaSinner in [#185](https://github.com/holochain/wind-tunnel/pull/185)
- Add `ci_pass` check (#183) by @ThetaSinner in [#183](https://github.com/holochain/wind-tunnel/pull/183)

### Removed

- Remove empty changelog headings and add missing release

### First-time Contributors

* @zippy made their first contribution in [#182](https://github.com/holochain/wind-tunnel/pull/182)

## \[[0.4.0-dev.1](https://github.com/holochain/wind-tunnel/compare/0.2.0-alpha.2...v0.4.0-dev.1)\] - 2025-06-19

### Added

- Exposed `on_signal` from the app websocket in the instrumented websocket.
- New handler function `handle_api_err` which can be used with `map_err` to deal with `ConductorApiError`s and convert
  them into `anyhow` errors or panic when the error is fatal.
- New common helper `uninstall_app`, see its rustdoc for details.
- Each run will now generate a unique run ID which is used to keep report data separate between runs. At some point it
  will be possible to specify a run ID to use but for now it is generated automatically.
- Check in the `happ_builder` whether `hc` and `cargo` are available. This is used by the scenario build script to skip
  building happs if build tools are not available. This allows the project to be loaded in an environment where the
  tools aren't available.
- A new tool for summarising scenario outcomes. This is called the `summariser` which is possibly a working title! The
  tool is specific to the scenarios in this project but does have some re-usable pieces. It remains to be decided whether
  we will separate those parts out and publish them as a crate. For now, this is private to the project.
- `run_with_required_agents` function for TryCP scenarios that fails if the number of agents that completed the scenario
  is less than the passed `min_required_agents`. Can be overridden with the `MIN_REQUIRED_AGENTS` environment variable.
- Check that the scenarios have a cargo package name that matches the directory name used by Nix packages. Panic when
  building the scenario if they do not match. [#122](https://github.com/holochain/wind-tunnel/pull/122)
- Nix dev shell for kitsune scenarios.

### Changed

- Updated to Holochain `v0.4.2`
- Updated to new Holochain client version `v0.6.2`
- Replace `&mut self` with `&self` in admin and app instrumented websockets.
- `ShutdownHandle` now hides its implementation. It works the same way that it did but you can no longer access the
  broadcast channel that it uses internally. Shutdown failures used to panic but it a `ShutdownHandle` happens to not
  have any subscribers then that should not be considered a fatal error. It will now log a warning instead.
- Metrics now automatically include `run_id` and `scenario_name` tags.
- Update `trycp_client` and `trycp_api` dependencies to `v0.17.0`. [#117](https://github.com/holochain/wind-tunnel/pull/117)
- When making zome calls with the TryCP client bindings, the `agent` is now reported on the metrics, taken from the target
  cell_id for the call. For the wrapped `holochain_client`, this is only done when the call target is `CellId`. Or in
  other words, the `agent` is not reported when calling a clone cell.
- All metrics are now reported in seconds, as an `f64`. There were some types still using milliseconds which made reporting
  across scenarios more complex.
- Increased TryCP test scenario duration to 30s in CI [Test Workflow](.github/workflows/test.yaml).
- Use the new `AppBundleSource::Bytes` variant to bundle scenarios [#152](https://github.com/holochain/wind-tunnel/pull/152)
- Test workflow uses kitsune dev shell for kitsune scenario.
- Converted `validation_receipts` scenario to non-TryCP scenario to be run on the Nomad cluster. [#172](https://github.com/holochain/wind-tunnel/pull/172)

### Fixed

- Run the TryCP scenarios in the [Performance Workflow](.github/workflows/performance.yaml) on the Holo Ports defined in [targets.yaml](targets.yaml). [#117](https://github.com/holochain/wind-tunnel/pull/117)
- Fix Kitsune op store to always return all processed op ids. Previously ops processed multiple times would not be removed from the request queue. Duplicate ops are still not considered for reporting.

## \[[0.2.0-alpha.2](https://github.com/holochain/wind-tunnel/compare/0.2.0-alpha.1...0.2.0-alpha.2)\] - 2024-05-24

### Changed

- Updated Holochain version to 0.3.1-rc.0 and updated all other dependencies to their corresponding versions.

## \[[0.2.0-alpha.1](https://github.com/holochain/wind-tunnel/commits/0.2.0-alpha.1)\] - 2024-03-29

### Added

- A new option `--reporter` has been added to the scenario CLI. Run with `--help` to see available options. It defaults
  to the `in-memory` implementation which will print a basic report to the console.

### Changed

- **BREAKING** The `holochain_client_instrumented`, `holochain_wind_tunnel_runner` and zomes have been upgraded to use Holochain 0.3.
  Specifically everything has been bumped to the 0.3.0-beta-dev.43 release of Holochain.
  This marks the end of 0.2 support for Wind Tunnel.

### Removed

- The `--no-metrics` flag has been removed from the scenario CLI.

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

### Changed
- Updated to Holochain 0.4.0
- Updated to new Holochain client version 0.5.0-alpha.4 which allowed `&mut self` to be replaced with `&self` in admin
  and app instrumented websockets.
- `ShutdownHandle` now hides its implementation. It works the same way that it did but you can no longer access the 
  broadcast channel that it uses internally. Shutdown failures used to panic but it a `ShutdownHandle` happens to not
  have any subscribers then that should not be considered a fatal error. It will now log a warning instead.
- Metrics now automatically include `run_id` and `scenario_name` tags.
- Update `trycp_client` and `trycp_api` dependencies to `v0.17.0-dev.6`. [#117](https://github.com/holochain/wind-tunnel/pull/117)
- When making zome calls with the TryCP client bindings, the `agent` is now reported on the metrics, taken from the target
  cell_id for the call. For the wrapped `holochain_client`, this is only done when the call target is `CellId`. Or in 
  other words, the `agent` is not reported when calling a clone cell.
- All metrics are now reported in seconds, as an `f64`. There were some types still using milliseconds which made reporting
  across scenarios more complex.
- Increased TryCP test scenario duration to 30s in CI [Test Workflow](.github/workflows/test.yaml).

### Deprecated
### Removed
### Fixed
- Run the TryCP scenarios in the [Performance Workflow](.github/workflows/performance.yaml) on the Holo Ports defined in [targets.yaml](targets.yaml). [#117](https://github.com/holochain/wind-tunnel/pull/117)

### Security

## [0.2.0-alpha.2] - 2024-05-25

### Changed

- Updated Holochain version to 0.3.1-rc.0 and updated all other dependencies to their corresponding versions.

## [0.2.0-alpha.1] - 2024-03-29

### Added

- A new option `--reporter` has been added to the scenario CLI. Run with `--help` to see available options. It defaults
  to the `in-memory` implementation which will print a basic report to the console.

### Changed

- **BREAKING** The `holochain_client_instrumented`, `holochain_wind_tunnel_runner` and zomes have been upgraded to use Holochain 0.3.
  Specifically everything has been bumped to the 0.3.0-beta-dev.43 release of Holochain.
  This marks the end of 0.2 support for Wind Tunnel.

### Removed

- The `--no-metrics` flag has been removed from the scenario CLI.

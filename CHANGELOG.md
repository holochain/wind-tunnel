# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-07-15

### Added

- Add job to comment the changelog preview on PRs by @cdunster in [#221](https://github.com/holochain/wind-tunnel/pull/221)
- Add missing CACHIX_AUTH_TOKEN env to cachix push step by @cdunster in [#219](https://github.com/holochain/wind-tunnel/pull/219)
- Add holochain_serialized_bytes dependency by @cdunster in [#213](https://github.com/holochain/wind-tunnel/pull/213)
- Add support for running scenarios on Nomad cluster (#136) by @cdunster in [#136](https://github.com/holochain/wind-tunnel/pull/136)
- Add kitsune dev shell in nix by @jost-s
- Add readme by @jost-s in [#153](https://github.com/holochain/wind-tunnel/pull/153)
- Add method to get kitsune agent id by @jost-s
- Add kitsune documentation by @jost-s
- Add missing devShell packages by @cdunster in [#151](https://github.com/holochain/wind-tunnel/pull/151)
- Add kitsune binding & sample scenario by @jost-s in [#147](https://github.com/holochain/wind-tunnel/pull/147)
- Add rustfmt component for formatting by @cdunster in [#122](https://github.com/holochain/wind-tunnel/pull/122)
- Add run_with_required_agents function for TryCP scenarios by @cdunster
- Add readme for the summariser by @ThetaSinner
- Add summary tests for the remaining scenarios by @ThetaSinner
- Add trends and partition timings by @ThetaSinner
- Add test for app install large by @ThetaSinner
- Add first test for app install by @ThetaSinner
- Address code review comment by @neonphog in [#93](https://github.com/holochain/wind-tunnel/pull/93)
- Address code review comment by @neonphog
- Address code review comment by @neonphog
- Add self-hosted runner and manual execution of performance tests (#88) by @cdunster in [#88](https://github.com/holochain/wind-tunnel/pull/88)
- Add trycp common call_zome function by @neonphog in [#77](https://github.com/holochain/wind-tunnel/pull/77)
- Add smoke test for trycp_write_validated by @neonphog
- Add write_validated dashboard by @neonphog
- Add deploy config for influx by @ThetaSinner
- Add new `use_installed_app` helper by @ThetaSinner
- Add local validation scenario (#63) by @ThetaSinner in [#63](https://github.com/holochain/wind-tunnel/pull/63)
- Add new targets by @ThetaSinner
- Add targets by @ThetaSinner
- Add new dashboard by @ThetaSinner
- Add new scenarios to pipeline by @ThetaSinner
- Add signals scenario by @ThetaSinner
- Add write_query scenario by @ThetaSinner
- Add write_read scenario and always clean up by @ThetaSinner
- Add first call scenario, fix reporting shutdown and update dashboards by @ThetaSinner
- Support merge queues by @ThetaSinner
- Add new crate to readme by @ThetaSinner
- Add read (get) scenario by @ThetaSinner
- Add badges for published crates by @ThetaSinner
- Add badge by @ThetaSinner
- Add meta for publish by @ThetaSinner
- Add smoke test to the CI pipeline by @ThetaSinner
- Add TODOs by @ThetaSinner
- Add Holochain bindings for the runner by @ThetaSinner
- Support teardown hooks and shutdown by @ThetaSinner
- Add initial env by @ThetaSinner

### Changed

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
- Remove TryCP (#177) by @ThetaSinner in [#177](https://github.com/holochain/wind-tunnel/pull/177)
- Migrate validation_receipts scenario away from TryCP (#172) by @cdunster in [#172](https://github.com/holochain/wind-tunnel/pull/172)
- Update flake.lock file (#174) by @github-actions[bot] in [#174](https://github.com/holochain/wind-tunnel/pull/174)
- Wait for Holochain up before starting smoke tests (#176) by @ThetaSinner in [#176](https://github.com/holochain/wind-tunnel/pull/176)
- Migrate scenario to default holochain by @jost-s in [#173](https://github.com/holochain/wind-tunnel/pull/173)
- Migrate to holochain for nomad by @jost-s in [#171](https://github.com/holochain/wind-tunnel/pull/171)
- Switch two_party_countersigning from TryCP to Holochain (#170) by @ThetaSinner in [#170](https://github.com/holochain/wind-tunnel/pull/170)
- Add `try_wait_for_min_peers` to Holochain runner common functions (#169) by @ThetaSinner in [#169](https://github.com/holochain/wind-tunnel/pull/169)
- Update Cargo.lock file by @ThetaSinner in [#158](https://github.com/holochain/wind-tunnel/pull/158)
- Mention kitsune op store fix and kitsune dev shell by @jost-s in [#159](https://github.com/holochain/wind-tunnel/pull/159)
- Scenario uses kitsune dev shell by @jost-s
- Allow overriding `run_id` and use it as the `network_seed` (#150) by @cdunster in [#150](https://github.com/holochain/wind-tunnel/pull/150)
- Update changelog with missing entries and fix mentioned versions (#156) by @cdunster in [#156](https://github.com/holochain/wind-tunnel/pull/156)
- Update AppBundleSource to Bytes variant by @cdunster in [#152](https://github.com/holochain/wind-tunnel/pull/152)
- Update holochain crates to v0.4.2 by @cdunster
- Nix flake update by @cdunster
- Configure number of messages with env var by @jost-s
- Send multiple messages per interval and randomize interval by @jost-s
- Use WtOp in op store by @jost-s
- Enable saying of multiple messages at once by @jost-s
- Update Cargo.lock file by @ThetaSinner in [#143](https://github.com/holochain/wind-tunnel/pull/143)
- Update flake.lock file by @ThetaSinner in [#142](https://github.com/holochain/wind-tunnel/pull/142)
- Update Docker Compose deploy to use restart always by @ThetaSinner in [#127](https://github.com/holochain/wind-tunnel/pull/127)
- Follow conventional commit style in cargo update workflow by @cdunster in [#138](https://github.com/holochain/wind-tunnel/pull/138)
- Follow conventional commit style in flake update workflow by @cdunster
- Format YAML by @ThetaSinner in [#139](https://github.com/holochain/wind-tunnel/pull/139)
- Format TOML by @ThetaSinner
- Upgrade to Holochain 0.4.0 by @ThetaSinner
- Update flake.lock by @ThetaSinner in [#129](https://github.com/holochain/wind-tunnel/pull/129)
- Update Cargo.lock by @ThetaSinner in [#128](https://github.com/holochain/wind-tunnel/pull/128)
- Update flake.lock by @ThetaSinner in [#126](https://github.com/holochain/wind-tunnel/pull/126)
- Increase the duration of the validation_receipts scenario in CI by @cdunster
- Increase the duration of the remote_call_rate scenario in CI by @cdunster
- Update the changelog by @cdunster
- Panic scenario build if directory and package names mismatch by @cdunster
- Update dependency, framework and bindings versions by @ThetaSinner
- Update Nix in CI by @ThetaSinner
- Pin to Holonix for 0.4 and update inputs by @ThetaSinner
- Update to latest stable Rust by @ThetaSinner
- Log debug message when MIN_REQUIRED_AGENTS is not set by @cdunster in [#115](https://github.com/holochain/wind-tunnel/pull/115)
- Update changelog by @cdunster
- Log warning when MIN_REQUIRED_AGENTS invalid by @cdunster
- Increase TryCP test scenario duration to 30s in CI by @cdunster
- Update changelog by @cdunster
- Update CI config to match renamed role by @ThetaSinner in [#103](https://github.com/holochain/wind-tunnel/pull/103)
- Update changelog by @ThetaSinner
- Update framework/summary_model/src/lib.rs by @ThetaSinner
- Update framework/summary_model/src/lib.rs by @ThetaSinner
- Update summariser/src/main.rs by @ThetaSinner
- Format by @ThetaSinner
- Fix missing test data for first_call by @ThetaSinner
- Revert "Remove unused test data" by @ThetaSinner
- Fix test data name collisions by @ThetaSinner
- Update summariser/src/lib.rs by @ThetaSinner
- Update summariser/src/analyze.rs by @ThetaSinner
- Update summariser/README.md by @ThetaSinner
- Update test data after review changes by @ThetaSinner
- First round of review changes by @ThetaSinner
- Include test data files in workspace build by @ThetaSinner
- Format TOML by @ThetaSinner
- Update dashboards by @ThetaSinner
- Fix after rebase by @ThetaSinner
- Summary for dht sync lag by @ThetaSinner
- Lint by @ThetaSinner
- More docs about usage by @ThetaSinner
- Validation receipts summary by @ThetaSinner
- Two party countersigning analysis by @ThetaSinner
- TryCP write validated to respect partitioning by @ThetaSinner
- Tidy by @ThetaSinner
- Local signals test by @ThetaSinner
- Tidy up and add test for first call by @ThetaSinner
- Fetch test data for use in tests by @ThetaSinner
- Summary for zome call single value by @ThetaSinner
- Summary for write validated by @ThetaSinner
- Summary for write read by @ThetaSinner
- Summary for write query by @ThetaSinner
- Summary for validation receipts by @ThetaSinner
- Summary for write validated by @ThetaSinner
- Summary for single write many read by @ThetaSinner
- Summary for remote call rate by @ThetaSinner
- Summary for local signals by @ThetaSinner
- Summarise more scenarios by @ThetaSinner
- Initial summarizer tool by @ThetaSinner
- Basic data queries by @ThetaSinner
- Results analysis by @ThetaSinner
- Update the changelog by @cdunster in [#117](https://github.com/holochain/wind-tunnel/pull/117)
- Update trycp client and API versions by @cdunster
- Update scenarios/remote_signals/README.md by @neonphog
- Yaml fmt by @neonphog
- Apply suggestions from code review by @neonphog
- Rename scenario to remote_signals by @jost-s
- Apply suggestions from code review by @neonphog
- Revert "different name for zome vs scenario" by @neonphog
- Different name for zome vs scenario by @neonphog
- Woops, remove 's' from CI jobs as well by @neonphog
- Lint by @neonphog
- Ci by @neonphog
- Better timeout tracking by @neonphog
- Initial working remote-signal scenario by @neonphog
- Review changes by @ThetaSinner in [#89](https://github.com/holochain/wind-tunnel/pull/89)
- Run with DPKI disabled by @ThetaSinner
- Handle countersigning signals and update metrics + dashboard by @ThetaSinner
- Improve countersigning scenario by @ThetaSinner
- Update flake.lock (#102) by @github-actions[bot] in [#102](https://github.com/holochain/wind-tunnel/pull/102)
- Update weekly and ci improvements (#101) by @ThetaSinner in [#101](https://github.com/holochain/wind-tunnel/pull/101)
- Update flake.lock (#98) by @github-actions[bot] in [#98](https://github.com/holochain/wind-tunnel/pull/98)
- Update Holochain versions (#96) by @ThetaSinner in [#96](https://github.com/holochain/wind-tunnel/pull/96)
- Nixos and direnv setup (#87) by @cdunster in [#87](https://github.com/holochain/wind-tunnel/pull/87)
- Update all versions (#85) by @ThetaSinner in [#85](https://github.com/holochain/wind-tunnel/pull/85)
- Validation Receipts Scenario (#83) by @neonphog in [#83](https://github.com/holochain/wind-tunnel/pull/83)
- Run CI on macos (#80) by @ThetaSinner in [#80](https://github.com/holochain/wind-tunnel/pull/80)
- Separate run commands by @ThetaSinner
- Update docs by @ThetaSinner
- Initial working trycp version of write_validated scenario by @neonphog
- Update flake.lock by @ThetaSinner in [#78](https://github.com/holochain/wind-tunnel/pull/78)
- Format by @ThetaSinner
- Update dependencies by @ThetaSinner
- Update flake.lock by @ThetaSinner in [#75](https://github.com/holochain/wind-tunnel/pull/75)
- Background trycp by @ThetaSinner
- Format log by @ThetaSinner
- Format by @ThetaSinner
- Update to latest Holochain by @ThetaSinner
- Update Cargo.lock by @ThetaSinner in [#72](https://github.com/holochain/wind-tunnel/pull/72)
- Update flake.lock by @ThetaSinner in [#71](https://github.com/holochain/wind-tunnel/pull/71)
- Update Cargo.lock by @ThetaSinner in [#70](https://github.com/holochain/wind-tunnel/pull/70)
- Update flake.lock by @ThetaSinner in [#68](https://github.com/holochain/wind-tunnel/pull/68)
- More information from the runner by @ThetaSinner
- Nix updates and format by @ThetaSinner
- TryCP logging by @ThetaSinner
- Shorter filter to please pkill by @ThetaSinner
- Stop network services outside Nix shell by @ThetaSinner
- Allow new file by @ThetaSinner
- Use local network services for testing by @ThetaSinner
- Use sbd server by @ThetaSinner
- Make countersigning scenario non-enzymatic by @ThetaSinner
- Use latest trycp by @ThetaSinner
- Clippy fix by @ThetaSinner
- Flake update by @ThetaSinner
- Upgrade to 0.4 (#64) by @ThetaSinner in [#64](https://github.com/holochain/wind-tunnel/pull/64)
- Update Cargo.lock (#62) by @github-actions[bot] in [#62](https://github.com/holochain/wind-tunnel/pull/62)
- Update flake.lock (#61) by @github-actions[bot] in [#61](https://github.com/holochain/wind-tunnel/pull/61)
- Two party countersigning (#60) by @ThetaSinner in [#60](https://github.com/holochain/wind-tunnel/pull/60)
- Update Cargo.lock (#59) by @github-actions[bot] in [#59](https://github.com/holochain/wind-tunnel/pull/59)
- Update flake.lock (#58) by @github-actions[bot] in [#58](https://github.com/holochain/wind-tunnel/pull/58)
- Update usage doc by @ThetaSinner
- Update Cargo.lock (#57) by @github-actions[bot] in [#57](https://github.com/holochain/wind-tunnel/pull/57)
- Format by @ThetaSinner
- Always run teardown and don't fail the run if an agent thread fails by @ThetaSinner
- Give remote calls more time by @ThetaSinner
- Give app installation more time by @ThetaSinner
- Update flake.lock (#56) by @github-actions[bot] in [#56](https://github.com/holochain/wind-tunnel/pull/56)
- Format Nix changes by @ThetaSinner
- Force install sweep by @ThetaSinner
- Tidy up error handling by @ThetaSinner
- Create script to run commands on targets by @ThetaSinner
- Use CI shell on CI by @ThetaSinner
- Remote call rate (#51) by @ThetaSinner in [#51](https://github.com/holochain/wind-tunnel/pull/51)
- Update Cargo.lock by @ThetaSinner in [#47](https://github.com/holochain/wind-tunnel/pull/47)
- Integrate trycp for multi node testing (#46) by @ThetaSinner in [#46](https://github.com/holochain/wind-tunnel/pull/46)
- Update flake.lock by @ThetaSinner in [#41](https://github.com/holochain/wind-tunnel/pull/41)
- Update zomes/signal/coordinator/src/lib.rs by @ThetaSinner in [#39](https://github.com/holochain/wind-tunnel/pull/39)
- Fail uninstalls gracefully by @ThetaSinner
- Update changelog and docs by @ThetaSinner
- Format by @ThetaSinner
- Format by @ThetaSinner
- Update lock by @ThetaSinner
- Fix unit tests by @ThetaSinner
- Use latest client by @ThetaSinner
- Improve the zome call dashboard by @ThetaSinner
- Dedup dashboards by @ThetaSinner
- Test first call on CI by @ThetaSinner
- Clippy fix by @ThetaSinner
- Clippy fix by @ThetaSinner
- Check in CI by @ThetaSinner
- App install scenario by @ThetaSinner
- Update Cargo.lock (#40) by @github-actions[bot] in [#40](https://github.com/holochain/wind-tunnel/pull/40)
- Update flake.lock by @ThetaSinner in [#38](https://github.com/holochain/wind-tunnel/pull/38)
- Update changelog by @ThetaSinner
- Update to 0.3.1-rc.0 (#37) by @ThetaSinner in [#37](https://github.com/holochain/wind-tunnel/pull/37)
- Update Cargo.lock (#34) by @github-actions[bot] in [#34](https://github.com/holochain/wind-tunnel/pull/34)
- Update to weekly 0.3-48 (#32) by @ThetaSinner in [#32](https://github.com/holochain/wind-tunnel/pull/32)
- Update to weekly 0.3-46 (#26) by @ThetaSinner in [#26](https://github.com/holochain/wind-tunnel/pull/26)
- Update flake.lock (#25) by @github-actions[bot] in [#25](https://github.com/holochain/wind-tunnel/pull/25)
- Update Cargo.lock (#24) by @github-actions[bot] in [#24](https://github.com/holochain/wind-tunnel/pull/24)
- Update to weekly 0.3-45 (#23) by @ThetaSinner in [#23](https://github.com/holochain/wind-tunnel/pull/23)
- Update Cargo.lock (#21) by @github-actions[bot] in [#21](https://github.com/holochain/wind-tunnel/pull/21)
- Update flake.lock (#20) by @github-actions[bot] in [#20](https://github.com/holochain/wind-tunnel/pull/20)
- Update Cargo.lock (#19) by @github-actions[bot] in [#19](https://github.com/holochain/wind-tunnel/pull/19)
- Update flake.lock (#18) by @github-actions[bot] in [#18](https://github.com/holochain/wind-tunnel/pull/18)
- Prepare release by @ThetaSinner
- Switch to 0.3 (#17) by @ThetaSinner in [#17](https://github.com/holochain/wind-tunnel/pull/17)
- Wind tunnel recording (#14) by @ThetaSinner in [#14](https://github.com/holochain/wind-tunnel/pull/14)
- Update flake.lock (#13) by @github-actions[bot] in [#13](https://github.com/holochain/wind-tunnel/pull/13)
- Update flake.lock (#12) by @github-actions[bot] in [#12](https://github.com/holochain/wind-tunnel/pull/12)
- Bump versions for alpha.3 (#11) by @ThetaSinner in [#11](https://github.com/holochain/wind-tunnel/pull/11)
- Update README.md by @ThetaSinner
- Handle errors better than unwrapping (#8) by @ThetaSinner in [#8](https://github.com/holochain/wind-tunnel/pull/8)
- Update flake.lock (#10) by @github-actions[bot] in [#10](https://github.com/holochain/wind-tunnel/pull/10)
- Update flake_update.yaml by @ThetaSinner
- Update flake_update.yaml by @ThetaSinner
- Create flake update job (#9) by @ThetaSinner in [#9](https://github.com/holochain/wind-tunnel/pull/9)
- WindTunnel Scenario Packaging (#7) by @ThetaSinner in [#7](https://github.com/holochain/wind-tunnel/pull/7)
- Rename step by @ThetaSinner
- Publish influxdb while waiting for authors to do so by @ThetaSinner
- Bump versions by @ThetaSinner
- Try to get CI to clean up after itself by @ThetaSinner
- Kill sandbox between smoke tests by @ThetaSinner
- Fmt by @ThetaSinner
- Smoke test all scenarios by @ThetaSinner
- Cover everything with clippy in CI by @ThetaSinner
- Docs for custom metrics by @ThetaSinner
- Clean up by @ThetaSinner
- Custom metrics and dht sync lag scenario by @ThetaSinner
- Fix shellcheck for telegraf.sh by @ThetaSinner
- Metrics capture by @ThetaSinner
- Doc sweep by @ThetaSinner
- More details for running scenarios by @ThetaSinner
- Clean up scripting and document usage by @ThetaSinner
- Lint and format by @ThetaSinner
- Fix integration with influxdb by @ThetaSinner
- Catch first scenario up with runner helpers and add doc by @ThetaSinner
- Allow custom scenario values and add call_zome helper by @ThetaSinner
- Stop using curl, causes build issues by @ThetaSinner
- Flake update by @ThetaSinner
- Extract influx as script because it's got too big to be inline by @ThetaSinner
- Auto create variables and dashboards by @ThetaSinner
- More reliable metric pushes by @ThetaSinner
- Fix link formatting by @ThetaSinner
- Fix link formatting by @ThetaSinner
- Fix link formatting by @ThetaSinner
- Full features for syn by @ThetaSinner
- Fix build script to fix CI by @ThetaSinner
- Prepare for release by @ThetaSinner
- Format by @ThetaSinner
- Allow metrics to be disabled by @ThetaSinner
- Tidy by @ThetaSinner
- Push metrics with influxive by @ThetaSinner
- Clippy some more goodness by @ThetaSinner
- Configure influx server by @ThetaSinner
- Clippy and format to fix CI by @ThetaSinner
- Use published client by @ThetaSinner
- Allow behaviours to be configured from the CLI by @ThetaSinner
- Simple CI pipeline by @ThetaSinner
- Clean up lint issues by @ThetaSinner
- Fix doc test by @ThetaSinner
- Tests for runner hooks by @ThetaSinner
- More user rust docs by @ThetaSinner
- Rust docs for scenario authors by @ThetaSinner
- Docs for wind tunnel methodology and scenario writing by @ThetaSinner
- Format by @ThetaSinner
- Project layout docs by @ThetaSinner
- Extract macro for locating happs built by the scenario build script by @ThetaSinner
- Extract install app by @ThetaSinner
- Extract app ws setup by @ThetaSinner
- Show a progress bar during the run by @ThetaSinner
- Pass connection string rather than hard-coding it by @ThetaSinner
- Start report on new line by @ThetaSinner
- Better config for agent count by @ThetaSinner
- Pretty summary report which is easier to read by @ThetaSinner
- Monitor CPU usage by @ThetaSinner
- Fix report grouping by @ThetaSinner
- First test, reporting not working yet by @ThetaSinner
- First pass app installs by @ThetaSinner
- Build DNAs for use with scenarios by @ThetaSinner
- Initial reporting setup by @ThetaSinner
- Instrument the app interface by @ThetaSinner
- Finish instrumenting the admin interface by @ThetaSinner
- Integrate and instrument client (wip) by @ThetaSinner
- Control shutdown by @ThetaSinner
- Initial tooling by @ThetaSinner
- Init npm project by @ThetaSinner
- Initial commit by @ThetaSinner

### Fixed

- Process_incoming_ops returns ids of all ops by @jost-s
- Run unit tests directly with cargo by @cdunster
- Fixup! chore: Update to latest stable Rust by @ThetaSinner in [#121](https://github.com/holochain/wind-tunnel/pull/121)
- Fix broken type link in doc-comment by @cdunster
- Correct scenario name in performance workflow by @cdunster in [#119](https://github.com/holochain/wind-tunnel/pull/119)
- Don't use CI conductor config for TryCP performance tests by @cdunster
- Run TryCP tests on the targets in the targets.yaml file by @cdunster
- Rename remote_signal_coordinator by @jost-s
- Fix naming conflict by adding _scenario suffix by @neonphog
- Allow name to be derived from pname-version by @cdunster

### Removed

- Remove behavior option from cli by @jost-s
- Remove the number of seconds from the comment by @cdunster
- Remove extra `///` in doc-comment by @cdunster
- Remove test data that actually isn't used by @ThetaSinner
- Remove unused test data by @ThetaSinner
- Remove 's' from scenario directory path by @neonphog
- Remove custom app definition by @cdunster in [#118](https://github.com/holochain/wind-tunnel/pull/118)
- Remove problem hosts by @ThetaSinner
- Remove problem step for now by @ThetaSinner
- Remove feature no longer required for holochain_client by @ThetaSinner
- Remove unused dependencies by @ThetaSinner

## First-time Contributors

* @dependabot[bot] made their first contribution in [#212](https://github.com/holochain/wind-tunnel/pull/212)

* @cdunster made their first contribution in [#221](https://github.com/holochain/wind-tunnel/pull/221)

* @ThetaSinner made their first contribution

* @github-actions[bot] made their first contribution in [#191](https://github.com/holochain/wind-tunnel/pull/191)

* @jost-s made their first contribution in [#188](https://github.com/holochain/wind-tunnel/pull/188)

* @zippy made their first contribution in [#182](https://github.com/holochain/wind-tunnel/pull/182)

* @neonphog made their first contribution in [#93](https://github.com/holochain/wind-tunnel/pull/93)

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
- Updated to Holochain `v0.5.2`
- Updated to new Holochain client version `v0.7.0`
- Enable the former TryCP scenarios on the Nomad cluster CI: remote_call_rate, remote_signals and two_party_countersigning.

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

### Deprecated
### Removed
### Fixed
- Run the TryCP scenarios in the [Performance Workflow](.github/workflows/performance.yaml) on the Holo Ports defined in [targets.yaml](targets.yaml). [#117](https://github.com/holochain/wind-tunnel/pull/117)
- Fix Kitsune op store to always return all processed op ids. Previously ops processed multiple times would not be removed from the request queue. Duplicate ops are still not considered for reporting.

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

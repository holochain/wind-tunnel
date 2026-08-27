# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Features

- Add `peerkit_client_instrumented` and `peerkit_wind_tunnel_runner` crates, providing a Peerkit binding for wind-tunnel alongside the existing Holochain and Kitsune bindings.
- Add `peerkit_hole_punch` scenario, exercising Peerkit's hole punching and throughput via a deployed relay. Every node connects to up to `PEERKIT_MAX_PEERS` discovered peers at a time, sends `PEERKIT_MESSAGES_PER_PEER` messages of `PEERKIT_MESSAGE_BYTES` bytes to each, and disconnects, repeating on a `PEERKIT_CYCLE_INTERVAL_MS` interval. Peer identities are random per agent, so any number of agents may share the single `node` behaviour. Metrics record peer discovery time, connection type (direct/relayed), and send/receive batch durations and byte counts, alongside an error count by kind.
- Add a `peerkit` Nix devShell for local development against the Peerkit binding and scenario.

### Changed

- **BREAKING**: Rename the Nomad job template `nomad/run_scenario.tpl.hcl` to `nomad/holochain_scenario.tpl.hcl` and introduce a new `runtime` vars key to select the scenario runtime. Anything that renders Nomad job templates by path must be updated to the new filename.

## \[[0.7.0](https://github.com/holochain/wind-tunnel/compare/v0.6.0...v0.7.0)\] - 2026-06-10

### Features

- Add seed script for version compatibility table by @ThetaSinner
- Add a version table and automation to update it by @ThetaSinner
- Upgrade to Holochain 0.6.1 by @ThetaSinner
- Include zome calls made in unyt UI in transactor loop of both unyt wind tunnel scenarios by @jost-s in [#603](https://github.com/holochain/wind-tunnel/pull/603)
- Review summary visualiser by @jost-s in [#617](https://github.com/holochain/wind-tunnel/pull/617)
- Notify team via mattermost message if threefold node cancellation fails by @mattyg
- Use custom threefold deployment tool, not tfrobot, for dynamic deployments based on available nodes, prioritized by region by @mattyg
- Nomad job download github hosted artifacts from commit sha that they were deployed from by @mattyg
- Deploy 4 VMs per threefold node to increase likelihood of finding enough available by @mattyg
- Only use threefold node pool for canonical-scaled variants by @mattyg
- Avoid flaky failures of upload_metrics due to server 500 timeout errors by adding jitter, retry and splitting metrics into smaller chunks by @mattyg
- Canonical-scaled scenario variant for deploying to large scale of nodes with threefold by @mattyg
- *(readme)* `unyt_proposal` scenario by @veeso in [#601](https://github.com/holochain/wind-tunnel/pull/601)
- *(summariser)* Upgrade Holochain metrics with all available metrics accurately summarised for every scenario (#573) by @ThetaSinner in [#604](https://github.com/holochain/wind-tunnel/pull/604)
- Summarize and visualize unyt chain transaction scenarios by @jost-s in [#593](https://github.com/holochain/wind-tunnel/pull/593)
- Retry downloads of holochain bin and scenario bin by @mattyg in [#596](https://github.com/holochain/wind-tunnel/pull/596)
- *(nomad)* Support arbitrary env vars in scenario vars.json files (#540) by @veeso in [#577](https://github.com/holochain/wind-tunnel/pull/577)
  - Replace the hardcoded MIN_AGENTS template variable with a generic env object in vars.json files. Each key-value pair in the env object is injected into the Nomad task's env block via a gomplate range loop.
  - Add env object support to gomplate template (run_scenario.tpl.hcl) - Migrate unyt_chain_transaction: move min_agents to env.MIN_AGENTS,   add NUMBER_OF_LINKS_TO_PROCESS with default value - Add env vars to write_get_agent_activity_volatile (CONDUCTOR_* vars) - Add env object to validation_receipts (empty, supports NO_VALIDATION_COMPLETE) - Add env vars to remote_signals (SIGNAL_INTERVAL_MS, RESPONSE_TIMEOUT_MS) - Update README documentation with env object syntax and example
- *(nomad)* Get `peer_end_count` by summing `peer_end_count` from run summaries inside of allocations by their run_id by @veeso in [#581](https://github.com/holochain/wind-tunnel/pull/581)
- Improve debugging of summariser query failures by @ThetaSinner
- Update canonical scenario variables to increase the testing scale by @ThetaSinner
- Get Unyt durable object store URL and secret from env vars by @cdunster
- Use UNYT_ prefix on Unyt-only env var by @cdunster
- Add subdirectory for the Cloudflare durable object store by @cdunster
- Add a 0-arc scenario for the Unyt app by @veeso in [#554](https://github.com/holochain/wind-tunnel/pull/554)
- *(summary-visualizer)* Convert bytes to bytes unit (e.g. KB,MB,GB...) by @veeso in [#572](https://github.com/holochain/wind-tunnel/pull/572)
  - Added new gomplate helpers to fomrat bytes and bytes rate. Then I've added the template use to all of the bytes units in the current templates
- Add basic dashboard for write_get_agent_activity_volatile by @mattyg in [#535](https://github.com/holochain/wind-tunnel/pull/535)
- Basic summary visualiser for write_get_agent_activity_volatile scenario by @mattyg
- Summariser for write_get_agent_activity_volatile scenario by @mattyg
- Write_get_agent_activity_volatile scenario by @mattyg
- Shutdown and restart conductor functions for holochain binding by @mattyg
- Upgrade scenario metrics to provide better insights by @ThetaSinner
- Add host metrics to all scenario visualiser templates by @mattyg in [#566](https://github.com/holochain/wind-tunnel/pull/566)
- Serialize AnomalyStatus with 'type' field for enum tag by @mattyg in [#530](https://github.com/holochain/wind-tunnel/pull/530)
- Display host metrics for dht_sync_lag in summary-visualiser by @mattyg
- Summary visualiser template helper for host metrics data by @mattyg
- Add unyt_chain_transaction scenario by @cdunster
  - This scenario tests the basic transactions of a Unyt network.
- Add install_app_custom for fine-grained hApp installation by @cdunster
  - The existing install_app function now just calls this with default values.
- Add a script for removing unused summariser test files by @ThetaSinner
- Report host metrics in parallel with scenario metrics by @ThetaSinner
- Do not exclude first and last window for trend in standard_rate stats by @mattyg in [#531](https://github.com/holochain/wind-tunnel/pull/531)
- Do not exclude first and last element from each window in standard_timing_stats by @mattyg
- *(summariser)* Add P2P metrics to arc-related scenarios by @veeso in [#511](https://github.com/holochain/wind-tunnel/pull/511)
  - Collect hc.holochain_p2p.handle_request.duration and hc.holochain_p2p.request.duration metrics in mixed_arc and zero_arc scenario summarizers. Adds overall and partitioned (by tag/message_type) P2P performance statistics to enable analysis of network behavior.
  - Updated scenarios: - mixed_arc_get_agent_activity - mixed_arc_must_get_agent_activity - full_arc_create_validated_zero_arc_read - zero_arc_create_and_read - zero_arc_create_data - zero_arc_create_data_validated
- Use rust build script to build hApps in Nix derivations by @cdunster
  - Instead of repeating the code to build hApps in Nix, just rely on the rust build script to do it for us and copy the hApps from the happs dir.
- Constrain Nomad job to only run on clients with Nomad >= v1.11.0 by @cdunster in [#519](https://github.com/holochain/wind-tunnel/pull/519)
- Use the new secret block in Nomad job instead of template block by @cdunster
- Use new restricted InfluxDB token, providing only access to windtunnel bucket by @lucksus in [#509](https://github.com/holochain/wind-tunnel/pull/509)
- Add influx dashboard for holochain_p2p metrics by @mattyg
- If test data file in 2_query_results cannot be loaded, return a 'NoSeriesInResult' so it is treated as if the influx query was empty by @mattyg
- Set HOLOCHAIN_INFLUXIVE env variable on holochain conductor process by @mattyg
- Update summary-visualizer templates to include newly added holochain_p2p metrics by @mattyg
- Include holochain_p2p metrics in relevent scenario summarizers by @mattyg
- Summariser functions for querying holochain_p2p metrics by @mattyg
- Scenario template for `mixed_arc_get_agent_activity` by @lucksus
- Scenario template for `full_arc_create_validated_zero_arc_read` by @lucksus
- Map API errors to potential agent bail errors in admin_websocket by @cdunster in [#490](https://github.com/holochain/wind-tunnel/pull/490)
  - Meaning that any calls to the admin websocket that result in an error that should bail and stop the Wind Tunnel agent will now do so.
- Map API errors to potential agent bail errors in app_websocket by @cdunster
  - Meaning that any calls to the app websocket that result in an error that should bail and stop the Wind Tunnel agent will now do so.
- \[**BREAKING**\] Improve the behaviours field in the nomad template variable files (#487) by @veeso in [#487](https://github.com/holochain/wind-tunnel/pull/487)
  - Feat!: Improved the behaviours field in the Nomad template variable files
  - **Breaking Change**: Scenarios must migrated. In the PR a Python3 script to execute the migration has been provided
- Use Iroh in Kitsune2 by @cdunster
- Add support for fetching happs from url (#471) by @veeso in [#471](https://github.com/holochain/wind-tunnel/pull/471)
  - Feat: add support for fetching happs from url.

### Bug Fixes

- Address PR review comments by @ThetaSinner
- *(nomad)* Fallback failed `fetch_peer_end_count` to zero by @veeso in [#624](https://github.com/holochain/wind-tunnel/pull/624)
  - Instead of fallbacking on the `peer_count` value, which leads to wrong `end_peer_count`, we just fallback to zero if the fetch fails
- Filter query of nomad jobs to only actively running jobs to avoid massive json response which crashes the runner by @mattyg
- Handle collecting github run artifacts from matrix even when only one instance in matrix by @mattyg
- Run scenario binaries with nix-ld wrapper by @mattyg
- Unyt chain transaction scenario by @jost-s in [#611](https://github.com/holochain/wind-tunnel/pull/611)
- *(nomad)* Handle "No running jobs" text output from nomad job status by @veeso in [#586](https://github.com/holochain/wind-tunnel/pull/586)
  - Nomad ignores the -json flag when there are no jobs and returns plain text instead of an empty JSON array, which caused jq parse failures in count_eligible_nodes.sh. Treat this case as zero busy nodes.
- Don't assume the number of scenarios that ran when visualising by @ThetaSinner
- Framework runner does not exit if tasks fail to cooperate with the shutdown by @ThetaSinner in [#575](https://github.com/holochain/wind-tunnel/pull/575)
- Invalid metric logic and type in scenario `full_arc_create_validated_zero_arc_read` for metric `retrieval_error_count` by @ThetaSinner in [#569](https://github.com/holochain/wind-tunnel/pull/569)
- Metrics from last page of history in unyt scenario were not sent by @cdunster in [#549](https://github.com/holochain/wind-tunnel/pull/549)
- Avoid rounding issues in `standard_timing_stats` by @ThetaSinner in [#533](https://github.com/holochain/wind-tunnel/pull/533)
- Busy loops in `mixed_arc_must_get_agent_activity` and `write_validated_must_get_agent_activity`. by @ThetaSinner
- Snapshot test should only load exected output when asserting it by @mattyg in [#534](https://github.com/holochain/wind-tunnel/pull/534)
- Dont round column values before selecting those within 2std and 3std by @mattyg in [#532](https://github.com/holochain/wind-tunnel/pull/532)
- `write_validated_must_get_agent_activity` scenario failed to produce `write_validated_must_get_agent_activity_chain_len` metrics by @veeso in [#506](https://github.com/holochain/wind-tunnel/pull/506)
- HApps fetched from URL are placed in correct directory by @cdunster in [#513](https://github.com/holochain/wind-tunnel/pull/513)
  - Fetched hApps were placed in a sub-directory in the `happs/` directory under the hApp's name but they should be placed under the package name.
- Build_info field structure that is generated in CI to run_summary.jsonl by @mattyg in [#515](https://github.com/holochain/wind-tunnel/pull/515)
- HOLOCHAIN_BIN_URL env variable was not set in CI by @mattyg
- Warning and error in telegraf log resolved by @ThetaSinner
- HApps fetched from URL are now placed in a directory as required by @cdunster in [#510](https://github.com/holochain/wind-tunnel/pull/510)
  - Fetched hApps were placed directly in the `happs/` directory but then looked for in a sub-directory.
- Influxql queries wrap the tag name in quotes, to avoid conflicting with influxql syntax elements. Rename test data query results files, as the query hash has changed. by @mattyg in [#499](https://github.com/holochain/wind-tunnel/pull/499)
- Ensure local_signals metrics written on early exit (#496) by @lucksus in [#496](https://github.com/holochain/wind-tunnel/pull/496)
  - Fix: ensure local_signals metrics written on early exit
  - The local_signals scenario was failing in CI because metrics were only written at the end of the agent_behaviour function. If the scenario duration ended while waiting for signals (30s timeout), the function would be interrupted and metrics would never be written.
  - This fix implements a MetricsGuard using Rust's RAII pattern with Drop to ensure recv metrics are always written when the function exits, whether normally or through early termination. The send metric is now written immediately after the zome call completes.
- Use_influx script properly parses influx token (#497) by @mattyg in [#497](https://github.com/holochain/wind-tunnel/pull/497)

### Miscellaneous Tasks

- Reduce canonical node targets from 65 to 30 by @ThetaSinner in [#631](https://github.com/holochain/wind-tunnel/pull/631)
  - Several machines used for canonical runs are no longer available. Scale the 65-node variants down to 30 while preserving each documented behaviour ratio.
- Update flake.lock file by @ThetaSinner in [#627](https://github.com/holochain/wind-tunnel/pull/627)
- Update the CONTRIBUTING.md with shared content in [#620](https://github.com/holochain/wind-tunnel/pull/620)
- Fix the timeout in the wait_for_jobs script by @cdunster in [#576](https://github.com/holochain/wind-tunnel/pull/576)
  - The timeout was reset on every allocation so the total timeout became TIMOUT*number_of_allocations.
- *(deps)* Upgrade Holochain to 0.6.1-rc.4 by @ThetaSinner
  - Pre-requisite for #573. Updates CI workflows, README, flake.lock, and Cargo dependencies to holochain 0.6.1-rc.4.
- *(nomad)* Add a job_name to the job JSON files to separate jobs by @cdunster
  - The canonical and demo runs had the same job names so were overwriting each other causing allocations to be superseded or ignored when both CI jobs were run at the same time.
- Go back to the INFLUX_TOKEN secret as that has changed in Nomad by @cdunster in [#602](https://github.com/holochain/wind-tunnel/pull/602)
- Add require durable objects env vars to nix packages by @cdunster in [#570](https://github.com/holochain/wind-tunnel/pull/570)
- Add Unyt Durable Object store URL and secret to the Nomad jobs by @cdunster
- Add bash function to run a local Durable Objects store by @cdunster
- Add Unyt durable object store URL and secret to devShell by @cdunster
- Add nodeJS and wrangler to Nix devShell by @cdunster
  - Required for the Durable Object store.
- Update to Holochain 0.6.1-rc.3 by @ThetaSinner
- Update the Unyt hApp in the Unyt Chain Transaction scenario by @cdunster in [#560](https://github.com/holochain/wind-tunnel/pull/560)
- Add InfluxDB dashboard for unyt_chain_transaction scenario by @cdunster in [#512](https://github.com/holochain/wind-tunnel/pull/512)
- Add unyt_chain_transaction scenario to Nomad jobs by @cdunster
- Remove unused summariser test data files by @ThetaSinner in [#541](https://github.com/holochain/wind-tunnel/pull/541)
- Remove openssl static packages from Nix workspace by @cdunster
- Fetch required hApps in Nix code with fetchurl by @cdunster
  - Nix derivations cannot access the internet so the scenario_build script fails to fetch the required hApps. The script first checks if they exist so by fetching them with Nix we avoid the need to fetch them again in the build script.
- Move Nix workspace-specific fields outside of the commonArgs by @cdunster
- Remove unnecessary pkg-config from Nix workspace build by @cdunster
- Use v1.11.1 of the Nomad package by @cdunster
- Add new flake input for nixpkgs-unstable channel by @cdunster
- Nix flake update by @cdunster
- Clean up template code by @lucksus in [#476](https://github.com/holochain/wind-tunnel/pull/476)
- Remove precise requirement for `kitsune2_bootstrap_srv` version by @cdunster
  - The invalid verson of `0.4.0` has now been yanked so it won't be used by default.
- Flake update to a patched version of Kitsune2 by @cdunster
- Update to Holochain `v0.6.1-rc.0` and Kitsune2 `v0.4.0-dev.2` by @cdunster

### Build System

- Update kitsune to 0.4.0-dev7 by @veeso in [#607](https://github.com/holochain/wind-tunnel/pull/607)
- Use bootstrap-srv provided by holonix by @mattyg in [#525](https://github.com/holochain/wind-tunnel/pull/525)
- Upgraded to edition 2024 (#485) by @veeso in [#485](https://github.com/holochain/wind-tunnel/pull/485)

### CI

- Update Holochain actions by @ThetaSinner in [#640](https://github.com/holochain/wind-tunnel/pull/640)
- Remove the Claude PR review workflow by @cdunster in [#639](https://github.com/holochain/wind-tunnel/pull/639)
- Also rebuild scenarios if cannot be found in a release by @cdunster in [#613](https://github.com/holochain/wind-tunnel/pull/613)
- Search for scenario in latest 10 releases instead of just one by @cdunster
  - This means that if the latest release doesn't have the scenario then use the next latest with it.
- Allow forcing a rebuild of all scenarios when running manually by @cdunster
- Only rebuild scenarios if their applicable files have changed by @cdunster
  - Otherwise, use the scenario zip from the latest (pre-)release.
- Use PAT token when creating pre-release to trigger other workflows by @cdunster
- Cancel pre-release scenario build when new release overrides it by @cdunster in [#618](https://github.com/holochain/wind-tunnel/pull/618)
- Create pre-releases on pushes to main that aren't full releases by @cdunster
- Allow manually running workflow to publish scenarios to a release by @cdunster
- Publish all scenario zip files to new releases by @cdunster
- Cancel Nomad allocations if wait-for-jobs fails by @cdunster
- Remove unused influx-token input from composite action by @cdunster
- Cleanup Threefold wallet mnemonic by @cdunster
- Also use the global RUN_ID for matrix output key by @cdunster
- Also use the global RUN_ID for uploading artifacts by @cdunster
- Set RUN_ID once in Nomad actions using GITHUB_ENV by @cdunster
- Add log of how many Threefold nodes are available by @cdunster
- Specify scenarios to run in parent workflow, not in nomad-common, so different scenario variants can specify difference scenarios to run by @mattyg
- Set nomad CI concurrency group to a shared name to prevent concurrent run between canonical and demo workflows by @veeso in [#610](https://github.com/holochain/wind-tunnel/pull/610)
- Add defaults when saving and loading the matrix in case of cancel by @cdunster in [#590](https://github.com/holochain/wind-tunnel/pull/590)
- Cancel Nomad allocation after timeout by @cdunster
- Append the run_attempt to the RUN_ID by @cdunster
  - When re-running a CI workflow the run ID will change so a new Nomad allocation should always be created.
- Add step to cancel running Nomad allocations if CI is cancelled by @cdunster
- Add nomad dry-run step before running by @cdunster
- Log summariser run RAM usage with `time -v` by @veeso in [#594](https://github.com/holochain/wind-tunnel/pull/594)
- Use latest 0.6.1 RC release by @ThetaSinner in [#568](https://github.com/holochain/wind-tunnel/pull/568)
- Always run the wait jobs after running scenarios, don't skip if a scenario failed by @ThetaSinner
- Always wait for all allocations to finish by @ThetaSinner
- Run the Durable Objects store locally in Unyt smoke tests by @cdunster
- *(claude)* Fixed usage of sticky_comments by removing the github token. Added directives for claude reviewer to only report errors, bugs, etc very concisely by @veeso in [#571](https://github.com/holochain/wind-tunnel/pull/571)
- Run write_get_agent_activity_volatile scenario in CI by @mattyg
- Test influx scripts by @veeso in [#536](https://github.com/holochain/wind-tunnel/pull/536)
- Add unyt_chain_transaction scenario to Nomad jobs by @cdunster
- Add unyt_chain_transaction scenario to smoke tests by @cdunster
- Do not reschedule failed nomad jobs by @mattyg in [#544](https://github.com/holochain/wind-tunnel/pull/544)
- Nomad-common.yaml is reading `behaviours` from the scenario file, but it should actually read `assignments` by @veeso in [#537](https://github.com/holochain/wind-tunnel/pull/537)
- Claude code PR reviews by @mattyg in [#538](https://github.com/holochain/wind-tunnel/pull/538)
- Print wind tunnel version and holochain build info in ci by @mattyg
- Cleanup 2 smoke test scripts to follow pattern of other scenario smoke tests (#494) by @mattyg in [#494](https://github.com/holochain/wind-tunnel/pull/494)

### Testing

- Enable `say_something_to_other_chatter` since fixed with kitsune 0.4.0-dev.6 by @veeso
- Temporarily disable `say_something_to_other_chatter` because it's hanging in ci by @veeso in [#580](https://github.com/holochain/wind-tunnel/pull/580)
- Stable ordering in snapshot tests by @ThetaSinner
- Update summariser test data with new fields, but empty contents by @mattyg
- Re-enable summary-visualiser smoke test for validation_receipts scenario by @mattyg in [#507](https://github.com/holochain/wind-tunnel/pull/507)
- Fix potential error if crypto provider installed multiple times by @cdunster
  - Check if a default `CryptoProvider` is already installed before installing one in the Kitsune2 client unit test.
- Use unit test init pattern for `env_logger` in Kitsune2 client by @cdunster

### Refactor

- Format byte units by passing values directly to scalar template, rather than re-implementing its html by @mattyg
- Move the fetch hApp logic into a module by @cdunster in [#526](https://github.com/holochain/wind-tunnel/pull/526)
- Reduce the Nix path to the commonArgs src by @cdunster
- Remove unused Nix module arguments by @cdunster

### Styling

- Consistent width for all metric label columns by @mattyg in [#598](https://github.com/holochain/wind-tunnel/pull/598)
- Expand charts to fill available width, while keeping the pixels per second in the x axis consistent for all charts in a scenario by @mattyg
- Ensure y axis labels are not clipped by @mattyg
- Cleanup styling on p50, p95, p99 display and fix html linting issues by @mattyg
- All byte units use 'bibyte' unit labels rather than byte unit labels, i.e. MB -> MiB by @mattyg
- All scalar labels left-align by @mattyg
- Scalar label on separate line, SD label consistent with all others by @mattyg
- Format Nix code by @cdunster

### Documentation

- Link to important runs by @ThetaSinner in [#632](https://github.com/holochain/wind-tunnel/pull/632)
- Add upgrade steps for Holochain to the CLAUDE.md by @ThetaSinner in [#626](https://github.com/holochain/wind-tunnel/pull/626)
- Add new-scenario checklist to PR template and CLAUDE.md by @veeso in [#625](https://github.com/holochain/wind-tunnel/pull/625)
- Add job_name to the readme by @cdunster
- Update the readmes for Unyt scenarios to include durable objects by @cdunster
- Improve claude.md by @ThetaSinner in [#550](https://github.com/holochain/wind-tunnel/pull/550)
- Add CLAUDE.md by @ThetaSinner in [#545](https://github.com/holochain/wind-tunnel/pull/545)
- Update readme with the correct relay-url by @cdunster in [#483](https://github.com/holochain/wind-tunnel/pull/483)

### Automated Changes

- *(deps)* Bump cachix/cachix-action from 16 to 17 by @dependabot[bot] in [#589](https://github.com/holochain/wind-tunnel/pull/589)
- *(deps)* Bump holochain/actions/.github/workflows/changelog-preview-comment.yml by @dependabot[bot] in [#616](https://github.com/holochain/wind-tunnel/pull/616)
- *(deps)* Bump holochain/actions/.github/workflows/prepare-release.yml by @dependabot[bot] in [#615](https://github.com/holochain/wind-tunnel/pull/615)
- *(deps)* Bump holochain/actions/.github/workflows/publish-release.yml by @dependabot[bot] in [#614](https://github.com/holochain/wind-tunnel/pull/614)
- *(deps)* Bump holochain/actions/.github/workflows/prepare-release.yml by @dependabot[bot] in [#559](https://github.com/holochain/wind-tunnel/pull/559)
- *(deps)* Bump holochain/actions/.github/workflows/publish-release.yml by @dependabot[bot] in [#558](https://github.com/holochain/wind-tunnel/pull/558)
- *(deps)* Bump actions/upload-artifact from 6 to 7 by @dependabot[bot] in [#557](https://github.com/holochain/wind-tunnel/pull/557)
- *(deps)* Bump actions/download-artifact from 7 to 8 by @dependabot[bot] in [#556](https://github.com/holochain/wind-tunnel/pull/556)
- *(deps)* Bump holochain/actions/.github/workflows/changelog-preview-comment.yml by @dependabot[bot] in [#555](https://github.com/holochain/wind-tunnel/pull/555)
- *(deps)* Bump holochain/actions/.github/workflows/publish-release.yml by @dependabot[bot] in [#491](https://github.com/holochain/wind-tunnel/pull/491)
- *(deps)* Bump holochain/actions/.github/workflows/prepare-release.yml by @dependabot[bot] in [#493](https://github.com/holochain/wind-tunnel/pull/493)
- *(deps)* Bump holochain/actions/.github/workflows/changelog-preview-comment.yml by @dependabot[bot] in [#492](https://github.com/holochain/wind-tunnel/pull/492)

### Other Changes

- Remove seed script by @ThetaSinner in [#634](https://github.com/holochain/wind-tunnel/pull/634)
- # This is a combination of 2 commits. by @ThetaSinner
- Update telegraf/telegraf.host.conf by @ThetaSinner in [#523](https://github.com/holochain/wind-tunnel/pull/523)
- Test/summary visualiser test uses snapshot data (#498) by @mattyg in [#498](https://github.com/holochain/wind-tunnel/pull/498)

### First-time Contributors

- @ made their first contribution in [#620](https://github.com/holochain/wind-tunnel/pull/620)
- @lucksus made their first contribution in [#509](https://github.com/holochain/wind-tunnel/pull/509)

## \[[0.6.0](https://github.com/holochain/wind-tunnel/compare/v0.5.0...v0.6.0)\] - 2026-01-23

### Features

- Check summary html on pre commit (#397) by @pdaoust in [#397](https://github.com/holochain/wind-tunnel/pull/397)
- Remove extra filtering after fix in Holochain 0.6.x by @cdunster
  - See [holochain/holochain#4255](https://github.com/holochain/holochain/issues/4255).
- Update release url by @matthme
- Upgrade to Holochain version 0.6 by @ThetaSinner
- Scenario template for `mixed_arc_must_get_agent_activity`, updates to `write_validated_must_get_agent_activity` (#444) by @pdaoust in [#444](https://github.com/holochain/wind-tunnel/pull/444)

### Bug Fixes

- Update flake.lock only selectively to try and evade derivation error by @matthme

### Miscellaneous Tasks

- Remove invalid comment about getting links by @cdunster

### Build System

- Upgrade kitsune2 input to 0.3 in nix flake by @jost-s

### CI

- Fix pkill call in kitsune2 test by @cdunster in [#486](https://github.com/holochain/wind-tunnel/pull/486)
- Update `holochain-bin-url` in Nomad workflows to `v0.6.0` by @cdunster
- Run `archive_bundle` step in ci only if nix files changed (#473) by @veeso in [#473](https://github.com/holochain/wind-tunnel/pull/473)
- Fix reference to `holochain-bin-url` variable by @cdunster in [#470](https://github.com/holochain/wind-tunnel/pull/470)
  - The variable name has been changed but the reference to it was not updated to the new name.

### Refactor

- `get_peer_list_randomized` to reduce iterators by @cdunster
  - The code was unnecessarily iterating and collecting multiple times.

### Styling

- Format the `bytes` dev-dependency in the standard way by @cdunster in [#317](https://github.com/holochain/wind-tunnel/pull/317)

## \[[0.5.0](https://github.com/holochain/wind-tunnel/compare/v0.5.0-dev.0...v0.5.0)\] - 2026-01-16

### Features

- *(kitsune)* To_connection_string returns String as before by @cdunster
- *(metrics)* Integrated Host metrics into Nomad runs (#250) by @veeso in [#250](https://github.com/holochain/wind-tunnel/pull/250)
  - Removed sed and temp telegraf config; read RUN_ID from env (or default to ""). Added a new `RUN_SUMMARY_PATH` env variable to specify a different location for the run_summary.jsonl when reporting summaries
- *(nomad)* Add agent config for local development by @cdunster
- *(nomad)* Use Holochain binary from PATH if no download URL by @cdunster
- *(nomad)* Run scenario from bin dir as provided by zip file by @cdunster
- *(nomad)* Set holochain bin to be executable by @cdunster
- *(nomad)* Download the `holochain` binary from provided URL by @cdunster
  - Use example URL for testing
- *(nomad)* Don't start a sandboxed Holochain conductor by @cdunster
  - This is now done via Wind Tunnel itself.
- *(runner)* Canonicalize the conductor root path by @cdunster
- *(runner)* Use temp-dir for conductor root dir by @cdunster
- *(runner)* Clean conductor parent directories after error by @cdunster in [#260](https://github.com/holochain/wind-tunnel/pull/260)
- *(runner)* Use '127.0.0.1' for admin URL instead of localhost by @cdunster
- *(runner)* Add helper function to call common agent_setup functions by @cdunster
- *(runner)* Take conductor stdin to avoid deadlocks by @cdunster
- *(runner)* Remove parent directory on drop if empty by @cdunster
- *(runner)* Create parent directories for conductor root path by @cdunster
- *(runner)* Get OS to select a free port instead of a random u16 by @cdunster
- *(runner)* Remove random conductor password and hardcode it by @cdunster
- *(runner)* Add agent name to stdout logs from conductors by @cdunster
- *(runner)* Set conductor root path base on agent name by @cdunster
- *(runner)* Only run conductor if connection-string not set by @cdunster
  - Generate a random admin port and run a conductor in-process with an admin interface on that port.
- *(runner)* Make `connection_string` an optional CLI option by @cdunster
- *(runner)* Store admin_ws_url in `HolochainAgentContext` by @cdunster
- *(runner)* Store app_ws_url as a `SocketAddr` by @cdunster
- *(runner)* Cleanup conductor file on error by @cdunster
- *(runner)* Only run conductor internally if WT_HOLOCHAIN_PATH set by @cdunster
- *(runner)* Allow setting target arc factor from agent_setup by @cdunster
- *(runner)* Move holochain_runner to agent context instead of runner by @cdunster
  - This allows `run_holochain_conductor` to be called in `agent_setup` instead of `setup`.
- *(runner)* Move app_ws_url to agent context instead of runner by @cdunster
  - This allows `configure_app_ws_url` to be called in `agent_setup` instead of `setup`.
- *(runner)* Wait for Holochain conductor to be ready after started by @cdunster
- *(runner)* Directly run Holochain conductor instead of via sandbox by @cdunster
- *(runner)* Add config struct for Holochain sandbox by @cdunster
- *(runner)* Add process to wait for the Holochain conductor by @cdunster
- *(runner)* Generate a random password for each new sandbox by @cdunster
- *(runner)* Allow setting hc bin path with env var by @cdunster
- *(runner)* Take sandbox admin port from connection string by @cdunster
- *(runner)* Creating a sandbox no longer cleans existing ones by @cdunster
- *(runner)* Add hard-coded sandbox clean, create, and run logic by @cdunster
- *(socket)* Impl `ToSocketAddr` for `SocketAddr` by @cdunster
- *(summariser)* Added Host metrics to the summariser analysis (#255) by @veeso in [#255](https://github.com/holochain/wind-tunnel/pull/255)
- Remove influx-client reporter, which is dead code (#450) by @mattyg in [#450](https://github.com/holochain/wind-tunnel/pull/450)
- Summary visualiser documentation (#437) by @pdaoust in [#437](https://github.com/holochain/wind-tunnel/pull/437)
- Do not retry jobs or tasks when a task fails, instead fail the job immediately (#424) by @mattyg in [#424](https://github.com/holochain/wind-tunnel/pull/424)
- Add mixed arc must_get_agent_activity scenario (#398) by @matthme in [#398](https://github.com/holochain/wind-tunnel/pull/398)
- Add `write_validated_must_get_agent_activity` scenario template (#385) by @pdaoust in [#385](https://github.com/holochain/wind-tunnel/pull/385)
- Holochain version number in scenario run summary (#404) by @pdaoust in [#404](https://github.com/holochain/wind-tunnel/pull/404)
- Upload summary json to hetzner bucket (#403) by @mattyg in [#403](https://github.com/holochain/wind-tunnel/pull/403)
- Run nomad scenario on all node pools (#409) by @mattyg in [#409](https://github.com/holochain/wind-tunnel/pull/409)
- Add mixed arc get_agent_activity scenario (#392) by @matthme in [#392](https://github.com/holochain/wind-tunnel/pull/392)
- Add `app_install` scenario template (#371) by @pdaoust in [#371](https://github.com/holochain/wind-tunnel/pull/371)
- Add `zome_call_single_value` scenario template (#388) by @pdaoust in [#388](https://github.com/holochain/wind-tunnel/pull/388)
- Add `local_signals` scenario template (#376) by @pdaoust in [#376](https://github.com/holochain/wind-tunnel/pull/376)
- Add full arc create validated zero arc read scenario (#364) by @matthme in [#364](https://github.com/holochain/wind-tunnel/pull/364)
- Adjust dht_sync_lag nomad vars to leverage the available nodes (#370) by @matthme in [#370](https://github.com/holochain/wind-tunnel/pull/370)
- Add flag to summariser to optionally ignore errors (#360) by @matthme in [#360](https://github.com/holochain/wind-tunnel/pull/360)
- Add quick-start guide to README (#357) by @matthme in [#357](https://github.com/holochain/wind-tunnel/pull/357)
- Report all errors in summariser (#356) by @matthme in [#356](https://github.com/holochain/wind-tunnel/pull/356)
- Add zero arc create data scenario with validation (#345) by @matthme in [#345](https://github.com/holochain/wind-tunnel/pull/345)
- Add scenario template for `remote_call_rate` (#349) by @pdaoust in [#349](https://github.com/holochain/wind-tunnel/pull/349)
- Wind tunnel scenario summary visualiser (#327) by @pdaoust in [#327](https://github.com/holochain/wind-tunnel/pull/327)
- Add zero arc create and read scenario (#338) by @matthme in [#338](https://github.com/holochain/wind-tunnel/pull/338)
- Add the Holochain build info to Run Summary (#333) by @veeso in [#333](https://github.com/holochain/wind-tunnel/pull/333)
- Add mixed zero arc/full arc scenario (#318) by @matthme in [#318](https://github.com/holochain/wind-tunnel/pull/318)
- If the summary report was not generated, the job is considered failed by @mattyg in [#312](https://github.com/holochain/wind-tunnel/pull/312)
- Make 'duration' a required nomad variable by @mattyg in [#313](https://github.com/holochain/wind-tunnel/pull/313)
- Add holochain metrics to Summariser (#263) by @ddd-mtl in [#263](https://github.com/holochain/wind-tunnel/pull/263)
- Add call to create_and_run_sandbox in all HC scenario setups by @cdunster
- Add Holochain Metrics dashboards (#261) by @ddd-mtl in [#261](https://github.com/holochain/wind-tunnel/pull/261)
- Support for importing Holochain metrics into InfluxDB (#254) by @ddd-mtl in [#254](https://github.com/holochain/wind-tunnel/pull/254)
- New CLI tool `lp-tool` for processing InfluxDB line protocol files (#256) by @ddd-mtl in [#256](https://github.com/holochain/wind-tunnel/pull/256)
- Integrate Host Metrics (#246) by @veeso in [#246](https://github.com/holochain/wind-tunnel/pull/246)
  - Added telegraf configurations and scripts to import Host and scenario metrics by the last run_id. Added a telegraf agent configuration to write Host metrics to file
- Use job-level Nomad secret for `INFLUX_TOKEN` by @cdunster in [#233](https://github.com/holochain/wind-tunnel/pull/233)
  - Having a separate token per job would be too much maintenance when they can just share a single token.
- Created a Job per scenario using a template (#227) by @veeso in [#227](https://github.com/holochain/wind-tunnel/pull/227)
- Added Host metrics to Telegraf and InfluxDB (#230) by @veeso in [#230](https://github.com/holochain/wind-tunnel/pull/230)
- Distinguish full vs pr nomad runs (#412) by @mattyg in [#412](https://github.com/holochain/wind-tunnel/pull/412)
- Add (almost) all scenario templates (#395) by @pdaoust in [#395](https://github.com/holochain/wind-tunnel/pull/395)
- Add scenario template for `zero_arc_create_and_read` (#365) by @pdaoust in [#365](https://github.com/holochain/wind-tunnel/pull/365)
- Add scenario template for `zero_arc_create_data` (#351) by @pdaoust in [#351](https://github.com/holochain/wind-tunnel/pull/351)
- Add scenario template for `validation_receipts` (#350) by @pdaoust in [#350](https://github.com/holochain/wind-tunnel/pull/350)
- Add `write_validated_must_get_agent_activity` scenario (#282) by @mattyg in [#282](https://github.com/holochain/wind-tunnel/pull/282)
- Add `write_get_agent_activity` scenario (#277) by @mattyg in [#277](https://github.com/holochain/wind-tunnel/pull/277)
- Add custom metrics in summarizer (#279) by @mattyg in [#279](https://github.com/holochain/wind-tunnel/pull/279)

### Bug Fixes

- *(nix)* Correct runtime inputs for Nix scripts by @cdunster in [#285](https://github.com/holochain/wind-tunnel/pull/285)
- *(nix)* Fix the flake-parts pkgs module override by @cdunster
  - The module override needs to be in the attribute set and not in the let statement.
- *(nomad)* Add extra participating agent for two_party_countersigning by @cdunster
- *(nomad)* Change nomad upload metrics script to use lp-tool and influx write instead of telegraf (#262) by @veeso in [#262](https://github.com/holochain/wind-tunnel/pull/262)
- *(runner)* Get_peer_list_randomized includes other agent's info (#281) by @mattyg in [#281](https://github.com/holochain/wind-tunnel/pull/281)
- *(runner)* Drain conductor stdout even if not printing by @cdunster
- *(runner)* Error if WT_HOLOCHAIN_PATH not set and bin not in PATH by @cdunster
- *(scripts)* Fix tq query command for INFLUX_TOKEN by @cdunster
- *(scripts)* Make all scripts executable by @cdunster
- *(summariser)* Format date-time in report name as NTFS-valid string by @cdunster
- Removed timeout logic from validation receipts scenario (#459) by @veeso in [#459](https://github.com/holochain/wind-tunnel/pull/459)
- Validation_receipts scenario gets stuck (#447) by @veeso in [#447](https://github.com/holochain/wind-tunnel/pull/447)
- Create telegraf metrics output dir if not found, to avoid the task failing and restarting 2m later (#423) by @mattyg in [#423](https://github.com/holochain/wind-tunnel/pull/423)
- Run run-summary job even if individual scenarios fail (#421) by @mattyg in [#421](https://github.com/holochain/wind-tunnel/pull/421)
- Eliminate race condition in holochain_binary tests by @ThetaSinner in [#400](https://github.com/holochain/wind-tunnel/pull/400)
  - The test `test_should_get_default_holochain_path` was flaky due to race conditions caused by parallel test execution modifying shared global environment variables (WT_HOLOCHAIN_PATH_ENV and PATH).
  - When tests ran in parallel, one test could modify environment variables while another was executing, causing sporadic failures with the error: "Path to Holochain binary overwritten with 'WT_HOLOCHAIN_PATH=/non/existent/path/to/holochain' but that path doesn't exist"
- Filter links by agent in write validated must_get_agent_activity zome (#394) by @matthme in [#394](https://github.com/holochain/wind-tunnel/pull/394)
- Change holochain binary url to specific tag (#390) by @matthme in [#390](https://github.com/holochain/wind-tunnel/pull/390)
- Namespace all helpers defined locally in scenario templates (#369) by @pdaoust in [#369](https://github.com/holochain/wind-tunnel/pull/369)
- Added missing summariser report for remote_signals (#340) by @veeso in [#340](https://github.com/holochain/wind-tunnel/pull/340)
- Added `customHolochain` as a dependency to nix `rust-smoke-test` job. (#336) by @veeso in [#336](https://github.com/holochain/wind-tunnel/pull/336)
- Use `force_stop_scenario` if conductor fails to start (#332) by @veeso in [#332](https://github.com/holochain/wind-tunnel/pull/332)
- Increase timeout when waiting for conductor to be ready by @cdunster
- Path handling and directory creation for conductor root path by @cdunster
  - `canonicalize` fails if the path doesn't exist so do it after creating the directories. `fs::create_dir_all` should not fail if directories already exist so no need to check.
- Use bootstrap & signal servers compatible with holochain 0.5.x+ (#283) by @mattyg in [#283](https://github.com/holochain/wind-tunnel/pull/283)
- Generate jobs script failed if called without arguments (#243) by @veeso in [#243](https://github.com/holochain/wind-tunnel/pull/243)
- Fixed host dashboards to skip custom buckets (#237) by @veeso in [#237](https://github.com/holochain/wind-tunnel/pull/237)
- Telegraf.conf missing tail plugin (#238) by @veeso in [#238](https://github.com/holochain/wind-tunnel/pull/238)
  - The tail plugin was removed by mistake in a previous cleanup of the conf file, but should be there to report metrics when running scenarios
- Fix missing fixture (#288) by @mattyg in [#288](https://github.com/holochain/wind-tunnel/pull/288)

### Miscellaneous Tasks

- *(nix)* Add missing tomlq package to devShell by @cdunster
- *(nix)* Add missing openssl package to devShell by @cdunster
- *(nix)* Use Nix store paths for scripts by @cdunster
- *(nix)* Add package override for rustfmt in git-hooks.nix by @cdunster
- *(nix)* Git-hooks' nixpkgs inputs follows nixpkgs by @cdunster
- *(nix)* Use rust-overlay's default overlays instead of importing by @cdunster
- *(nomad)* Cleanup Nomad job spec variables (#274) by @veeso in [#274](https://github.com/holochain/wind-tunnel/pull/274)
  - Removed unnecessary variables that won't change between runs, i.e. scenario-name. Remove undesired defaults Update the gomplate comment about the defaults to remove the blank line in the generated file Rename all variables to use underscores instead of hyphens Update the Nomad CI workflow to use the new variable names Update README based on changes, i.e., no scenario name and different var names
- *(telegraf)* Changed local-telegraf and ci-telegraf scripts to use lp-tool and influx CLI to import metrics (#258) by @veeso in [#258](https://github.com/holochain/wind-tunnel/pull/258)
  - This is necessary because telegraf when run with --once and inputs.file caps the amount of metrics to 10k lines
- Fix margin between scenario summaries (#368) by @pdaoust in [#368](https://github.com/holochain/wind-tunnel/pull/368)
- Remove redundant missing value checks, standardise on `default` rather than `or` (#363) by @pdaoust in [#363](https://github.com/holochain/wind-tunnel/pull/363)
- Update Cargo.lock file by @ThetaSinner in [#248](https://github.com/holochain/wind-tunnel/pull/248)
- Update flake.lock file by @ThetaSinner in [#273](https://github.com/holochain/wind-tunnel/pull/273)
- Add nix directory to direnv watch list by @cdunster
- Add .direnv directory to gitignore by @cdunster
  - This stops statix from checking it.
- Review step 1 by @veeso
- Review by @veeso
- Refactor nix flake by @veeso
  - Added rust-toolchain
- Fix wrong field name in expect log by @cdunster
- Have nomad support Holochain metrics by @ddd-mtl in [#265](https://github.com/holochain/wind-tunnel/pull/265)
- Update flake.lock file by @ThetaSinner in [#251](https://github.com/holochain/wind-tunnel/pull/251)
- Update dashboards (#242) by @veeso in [#242](https://github.com/holochain/wind-tunnel/pull/242)

### Build System

- Fix issue where scenarios failed to execute when built from a nixos system with an updated flake lock, because the nix store path for the interpreter had changed (#439) by @mattyg in [#439](https://github.com/holochain/wind-tunnel/pull/439)
- Bump rust 1.90 (#319) by @mattyg in [#319](https://github.com/holochain/wind-tunnel/pull/319)

### CI

- *(nomad)* Run summariser step even if there are failures by @cdunster in [#331](https://github.com/holochain/wind-tunnel/pull/331)
  - Otherwise if a single scenario fails the summariser report isn't produced.
- *(nomad)* Generate a single Summariser report for all scenarios by @cdunster
- *(nomad)* Fix JSON in matrix persist step to be strings by @cdunster in [#325](https://github.com/holochain/wind-tunnel/pull/325)
  - In the rare chance that one of the fields is a valid number then the JSON type would be a number.
- *(nomad)* Add 30 minute timeout for waiting for free nodes by @cdunster
- *(nomad)* Re-enable all scenarios by @cdunster
- *(nomad)* Add the job name to the summariser report name by @cdunster
- *(nomad)* Fix the required-nodes for two_party_countersigning by @cdunster
- *(nomad)* Disable the write_read scenario by @cdunster
- *(nomad)* Disable the single_write_many_read scenario by @cdunster
- *(nomad)* Disable the write_get_agent_activity scenario by @cdunster
- *(nomad)* Disable the app_install scenarios by @cdunster
- *(nomad)* Disable the validation_receipts scenario by @cdunster
- *(nomad)* Disable the two_party_countersigning scenario by @cdunster
- *(nomad)* Set timeout for waiting for jobs to finish to 90 minutes by @cdunster
- *(nomad)* Run the Summariser after running nomad scenarios (#278) by @veeso in [#278](https://github.com/holochain/wind-tunnel/pull/278)
- *(nomad)* Update default holochain_bin_url to official latest release by @cdunster
- *(nomad)* Remove required and default from holochain_bin_url input by @cdunster
- *(nomad)* Add default for NOMAD_VAR_holochain_bin_url by @cdunster
  - When running on schedule or without workflow_dispatch the inputs are all null.
- *(nomad)* Add holochain_bin_url input with default by @cdunster
- *(nomad)* Upload artifact of scenario bin and happs instead of bundle by @cdunster
- Log nomad output to debug exit code 2 failure on run_nomad (#453) by @veeso in [#453](https://github.com/holochain/wind-tunnel/pull/453)
- Run the test workflow on legacy `main-*` branches by @cdunster in [#466](https://github.com/holochain/wind-tunnel/pull/466)
- Increase timeout buffer for Nomad jobs by @cdunster in [#446](https://github.com/holochain/wind-tunnel/pull/446)
  - Uploading the metrics sometimes takes more than 5 minutes so a buffer of 5 minutes is not long enough to ensure the job finishes.
- Remove self-hosted github action runner (#433) by @mattyg in [#433](https://github.com/holochain/wind-tunnel/pull/433)
- Increase time buffer when timing out an allocation by @ThetaSinner in [#414](https://github.com/holochain/wind-tunnel/pull/414)
- Automate generation of summary visualizer for nomad (#374) by @veeso in [#374](https://github.com/holochain/wind-tunnel/pull/374)
- Disable validation_receipts scenario (#373) by @veeso in [#373](https://github.com/holochain/wind-tunnel/pull/373)
- Change Nomad scenario timeout to `(created_at - t_now) + duration + buffer` (#355) by @veeso in [#355](https://github.com/holochain/wind-tunnel/pull/355)
- Cache summariser build (#344) by @veeso in [#344](https://github.com/holochain/wind-tunnel/pull/344)
- Run nomad ci workflow if there are relevant files changed (#328) by @veeso in [#328](https://github.com/holochain/wind-tunnel/pull/328)
  - Automatically runs the nomad ci workflow if either the nomad workflow file or any scenario file changed
- Replace default Holochain bin URL with go-pion-unstable version by @cdunster
  - A build with unstable features enabled is required for Wind Tunnel scenarios.
- Don't add changelog preview comment to dependabot PRs by @cdunster in [#293](https://github.com/holochain/wind-tunnel/pull/293)
- Upload run_summary artifact to github ci by @veeso in [#286](https://github.com/holochain/wind-tunnel/pull/286)
- Kitsune by @veeso
- Tests by @veeso
- Moved rust toolchain to nix rust module by @veeso
- Taplo by @veeso
- Remove running of hc sandbox in scenario tests by @cdunster
- Run Nomad workflow once a week on Thu (#276) by @veeso in [#276](https://github.com/holochain/wind-tunnel/pull/276)
  - New HC version is released on Wednesday, so we can run the Nomad workflow with the latest release once a week
- Parallelise the running of nomad jobs (#272) by @veeso in [#272](https://github.com/holochain/wind-tunnel/pull/272)
- Removed performance workflow (#275) by @veeso in [#275](https://github.com/holochain/wind-tunnel/pull/275)
  - Removed the performance workflow and the ci-upload-metrics script.
- Wait for Nomad CI jobs to run (#245) by @veeso in [#245](https://github.com/holochain/wind-tunnel/pull/245)
- Use the same branch name for every nix flake update by @cdunster in [#247](https://github.com/holochain/wind-tunnel/pull/247)
- Use the same branch name for every cargo update by @cdunster
- Fix step that builds Nomad job file to use job-name if exists by @cdunster

### Testing

- Add snapshot tests to zero arc create data validated test (#366) by @matthme in [#366](https://github.com/holochain/wind-tunnel/pull/366)

### Refactor

- *(nix)* Remove unnecessary shellcheck packages from scripts by @cdunster
- *(nix)* Use git-hooks.nix enabledPackages by @cdunster
  - Instead of manually including all the packages again.
- *(nix)* Move setting of formatter out of module by @cdunster
- *(runner)* Check WT_HOLOCHAIN_PATH env var first by @cdunster
- *(runner)* All HolochainConfigBuilder methods take `&mut self` by @cdunster
  - This keeps the builder API consistent as other methods need to be called with a mut ref.
- *(runner)* Only set holochain bin path if env var is set by @cdunster
- Upload scenario bin to bucket, delete after scenario completes (#434) by @mattyg in [#434](https://github.com/holochain/wind-tunnel/pull/434)
- Remove countersigning scenario from CI workflows (#415) by @matthme in [#415](https://github.com/holochain/wind-tunnel/pull/415)
- Move code to get holochain build info from framework runner to holochain bindings (#396) by @mattyg in [#396](https://github.com/holochain/wind-tunnel/pull/396)
- Github workflow 'nomad' now runs summarizer for each job individually, rather than waiting for all jobs to succeed before running summarizer by @mattyg
- Call helper function in scenario to start the conductor by @cdunster
- Snake-case everything (#362) by @pdaoust in [#362](https://github.com/holochain/wind-tunnel/pull/362)
- Make CSS reusable (#361) by @pdaoust in [#361](https://github.com/holochain/wind-tunnel/pull/361)
- Refactor summary visualiser tests (#352) by @pdaoust in [#352](https://github.com/holochain/wind-tunnel/pull/352)

### Styling

- *(nix)* Remove whitespace from the start of empty lines by @cdunster
- *(runner)* Reformat the holochain runner code by @cdunster
- Format influx templates (#239) by @veeso in [#239](https://github.com/holochain/wind-tunnel/pull/239)
  - In order to catch changes when working on PRs, let's prettify template json files

### Documentation

- *(nomad)* Update the scenario_url description by @cdunster in [#284](https://github.com/holochain/wind-tunnel/pull/284)
- *(runner)* Add details to `run_holochain_conductor` doc-comment by @cdunster
- *(runner)* Add doc-comments to `run_holochain_conductor` function by @cdunster
- *(runner)* Add doc-comments to the holochain_runner module by @cdunster
- Update 0.5.0-dev.0 release in changelog to match template by @cdunster in [#465](https://github.com/holochain/wind-tunnel/pull/465)
  - Only do this version because it was the first version to start using conventional commits and a generated changelog.
- Add missing footer to changelog by @cdunster
- Update changelog first-time heading to match template by @cdunster
- Update changelog version headings to match template by @cdunster
- Update changelog header to match template by @cdunster
- Add GitHub issue numbers to the todos for fixing the scenarios by @cdunster in [#295](https://github.com/holochain/wind-tunnel/pull/295)
- Update the README with how to run with Holochain binary by @cdunster
- Update section in README about generating Nomad job files by @cdunster
- Reword section about connecting to an external conductor by @cdunster
- Fix punctuation in README by @cdunster
- Fix copy-paste error in doc-comment by @cdunster
- Add paragram in project README about conductor stdout and logs by @cdunster
- Update default run examples in all scenario READMEs by @cdunster
- Update project readme to use in-process conductors by @cdunster
- Move conventional commits remark in PR template to one line by @cdunster in [#229](https://github.com/holochain/wind-tunnel/pull/229)
  - GitHub appears to not respect markdown properly and so was adding linebreaks to this remark.

### Automated Changes

- *(deps)* Bump actions/upload-artifact from 5 to 6 (#407) by @dependabot[bot] in [#407](https://github.com/holochain/wind-tunnel/pull/407)
- *(deps)* Bump actions/download-artifact from 5 to 7 (#408) by @dependabot[bot] in [#408](https://github.com/holochain/wind-tunnel/pull/408)
- *(deps)* Bump actions/cache from 4 to 5 by @dependabot[bot] in [#405](https://github.com/holochain/wind-tunnel/pull/405)
- *(deps)* Bump peter-evans/create-pull-request from 7 to 8 by @dependabot[bot] in [#406](https://github.com/holochain/wind-tunnel/pull/406)
- *(deps)* Bump actions/upload-artifact from 4 to 5 (#315) by @dependabot[bot] in [#315](https://github.com/holochain/wind-tunnel/pull/315)
- *(deps)* Bump actions/download-artifact from 5 to 6 (#314) by @dependabot[bot] in [#314](https://github.com/holochain/wind-tunnel/pull/314)
- *(deps)* Bump holochain/actions from 1.2.0 to 1.3.0 by @dependabot[bot] in [#290](https://github.com/holochain/wind-tunnel/pull/290)
- *(deps)* Bump actions/download-artifact from 4 to 5 by @dependabot[bot] in [#280](https://github.com/holochain/wind-tunnel/pull/280)
- *(deps)* Bump actions/checkout from 4 to 5 by @dependabot[bot] in [#252](https://github.com/holochain/wind-tunnel/pull/252)
- Update Cargo.lock file (#326) by @github-actions[bot] in [#326](https://github.com/holochain/wind-tunnel/pull/326)
- Update flake.lock file (#320) by @github-actions[bot] in [#320](https://github.com/holochain/wind-tunnel/pull/320)
- Update flake.lock file (#298) by @github-actions[bot] in [#298](https://github.com/holochain/wind-tunnel/pull/298)

### First-time Contributors

- @veeso made their first contribution in [#453](https://github.com/holochain/wind-tunnel/pull/453)
- @mattyg made their first contribution in [#450](https://github.com/holochain/wind-tunnel/pull/450)
- @pdaoust made their first contribution in [#437](https://github.com/holochain/wind-tunnel/pull/437)
- @matthme made their first contribution in [#398](https://github.com/holochain/wind-tunnel/pull/398)
- @ddd-mtl made their first contribution in [#263](https://github.com/holochain/wind-tunnel/pull/263)

## \[[0.5.0-dev.0](https://github.com/holochain/wind-tunnel/compare/v0.4.0-dev.1...v0.5.0-dev.0)\] - 2025-07-16

### Features

- Update to use holochain  0.5 by @zippy in [#182](https://github.com/holochain/wind-tunnel/pull/182)

### Miscellaneous Tasks

- Prepare next release by @cdunster in [#224](https://github.com/holochain/wind-tunnel/pull/224)
- Add `holochain_serialized_bytes` dependency by @cdunster in [#213](https://github.com/holochain/wind-tunnel/pull/213)
  - Required with the latest version of holochain.
- Use workspace package properties (#198) by @ThetaSinner in [#198](https://github.com/holochain/wind-tunnel/pull/198)
- Maintenance update versions by @ThetaSinner in [#192](https://github.com/holochain/wind-tunnel/pull/192)

### CI

- Add job to comment the changelog preview on PRs by @cdunster in [#221](https://github.com/holochain/wind-tunnel/pull/221)
  - Only run the job on PRs and only PRs that don't have the `hra-release` label as these are the PRs that generate the real changelog.
- Add missing `CACHIX_AUTH_TOKEN` env to cachix push step by @cdunster in [#219](https://github.com/holochain/wind-tunnel/pull/219)
- Add release support by @ThetaSinner in [#193](https://github.com/holochain/wind-tunnel/pull/193)
- Enable scenarios `remote_call_rate`, `remote_signals` & `two_party_countersigning` on nomad cluster by @jost-s in [#188](https://github.com/holochain/wind-tunnel/pull/188)
- Track and reduce disk usage (#189) by @ThetaSinner in [#189](https://github.com/holochain/wind-tunnel/pull/189)
- Use less disk space (#185) by @ThetaSinner in [#185](https://github.com/holochain/wind-tunnel/pull/185)
- Add `ci_pass` check (#183) by @ThetaSinner in [#183](https://github.com/holochain/wind-tunnel/pull/183)

### Documentation

- Markdown format the CHANGELOG.md by @cdunster
- Remove empty changelog headings and add missing release by @cdunster
- Update PR template for conventional commits usage by @cdunster

### Automated Changes

- *(deps)* Bump holochain/actions from 1.0.0 to 1.2.0 by @dependabot[bot] in [#212](https://github.com/holochain/wind-tunnel/pull/212)
- *(deps)* Bump AdityaGarg8/remove-unwanted-software from 2 to 5 by @dependabot[bot] in [#195](https://github.com/holochain/wind-tunnel/pull/195)
- *(deps)* Bump peter-evans/create-pull-request from 6 to 7 (#194) by @dependabot[bot] in [#194](https://github.com/holochain/wind-tunnel/pull/194)

### First-time Contributors

- @dependabot[bot] made their first contribution in [#212](https://github.com/holochain/wind-tunnel/pull/212)
- @zippy made their first contribution in [#182](https://github.com/holochain/wind-tunnel/pull/182)

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

<!-- generated by git-cliff -->

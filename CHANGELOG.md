# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## \[[0.6.0](https://github.com/holochain/wind-tunnel/compare/v0.5.0...v0.6.0)\] - 2026-01-23

### Features

- Check summary html on pre commit (#397) by @pdaoust in [#397](https://github.com/holochain/wind-tunnel/pull/397)
  - Feat: check summary visualiser templates and HTML output on commit, when either the templates, the summary visualiser tool itself, or the flake file have changed
- Remove extra filtering after fix in Holochain 0.6.x by @cdunster
  - See https://github.com/holochain/holochain/issues/4255.
- Update release url by @matthme
- Upgrade to Holochain version 0.6 by @ThetaSinner
- Scenario template for `mixed_arc_must_get_agent_activity`, updates to `write_validated_must_get_agent_activity` (#444) by @pdaoust in [#444](https://github.com/holochain/wind-tunnel/pull/444)
  - Chore: refactor templates for reuse with must_get_agent_acitivty scenarios * feat: scenario template for mixed_arc_must_get_agent_activity

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
  - Ci: Run `archive_bundle` step in ci only if nix files changed
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

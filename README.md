# Wind Tunnel

[![test](https://github.com/holochain/wind-tunnel/actions/workflows/test.yaml/badge.svg)](https://github.com/holochain/wind-tunnel/actions/workflows/test.yaml)

Performance testing for Holochain, modelled as load tests. The name is a reference to aerodynamics testing and is a good
way to refer to this project but the language does not extend to the code.

### Navigating the Project

#### The Wind Tunnel Framework

The `wind-tunnel` framework, which is found in `./framework`, is a collection of crates that implement the core logic for 
running load tests and collecting results. This code is not specific to Holochain and should stay that way. It provides extension
points for hooking in Holochain specific behaviour.

It is divided into the following crates:
- `wind_tunnel_instruments`: Tools for collecting and reporting metrics.
- `wind_tunnel_instruments_derive`: Procedural macros to generate code that helps integrate the `wind_tunnel_instruments` crate.
- `wind_tunnel_runner`: The main logic for running load tests. This is a library that is designed to be embedded inside a test binary.

#### Holochain Bindings for Wind Tunnel

The `bindings`, found in `./bindings`, customise the `wind-tunnel` framework to Holochain. They are what you would be
consuming when using `wind-tunnel` to test Holochain.

The bindings contain the following crates:
- `holochain_client_instrumented`: A wrapper around the [`holochain_client`](https://crates.io/crates/holochain_client) that uses `instruments` and `instruments_derive`
  to instrument the client. It exports an API that is nearly identical to the `holochain_client` crate, except that when constructing client connections
  you need to provide a reporter which it can write results to.
- `holochain_wind_tunnel_runner`: This is a wrapper around the `wind_tunnel_runner` crate that provides Holochain specific code to be used with the `wind_tunnel_runner`.
  The `wind_tunnel_runner` is re-exported, so you should just use this crate as your runner when creating tests for Holochain.

#### Kitsune Bindings for Wind Tunnel

[Kitsune](https://github.com/holochain/kitsune2) is a framework for writing distributed hash table based network applications. Bindings for Wind Tunnel enable developers
to write scenarios to test Kitsune.

The bindings contain the following crates:
- `kitsune_client_instrumented`: A test application specifically written for Wind Tunnel. With Kitsune at its core, instances are created to publish text messages which
are sent to all participating peers. The API is minimal, as the focus lies on observing speed and resilience of delivering messages to peers.
- `kitsune_wind_tunnel_runner`: A wrapper around the `wind_tunnel_runner` crate that provides Kitsune specific code to be used with the `wind_tunnel_runner`. It provides
CLI options to configure scenarios.

#### Scenarios

The `scenarios`, found in `./scenarios`, are what describe the performance testing scenarios to be run against Holochain. Each scenario
is a binary that uses the `holochain_wind_tunnel_runner` as a library. When it is run it will have all the capabilities that `wind-tunnel` provides.

There is more information about how to create scenarios in a [separate section](#writing-scenarios-for-holochain).

#### Creating hApps for use in scenarios

> [!NOTE]
> This section is optional, you can skip it if you are working outside this repository or choose your own hApp packaging strategy.

When a scenario is run, it may install a hApp from any place or using any method that Holochain supports. While working in this 
repository, there are some helpers to make this easier.

The `zomes` directory contains Rust projects that are intended to be built into zomes. Please check the directory structure and
naming of convention for existing zomes when adding new ones. In particular:
- Each zome should be in its own directory with a name that describes its purpose.
- Each zome should keep its coordinator and integrity zomes separate as Rust projects in `coordinator` and `integrity` directories.
- Each zome should reference the shared zome build script as `build = "../../wasm_build.rs"`
- The library that gets produced by the zome should be consistently named in the `[lib]` section as `<zome_name>_(coordinator|integrity)`.

When you want to use one or more zomes in a scenario, you should package them into a hApp for that scenario. To achieve this
your scenario needs to do three things:
1. Reference the custom build script which will package the zomes into a hApp for you as `build = "../scenario_build.rs"`
2. Add custom sections to the `Cargo.toml` to describe the hApps you need in your scenario. There is an example at the end of this section.
3. Reference the installed app from your scenario using the provided macro `scenario_happ_path!("<hApp name>")`. This produces a `std::path::Path`
   that can be passed to Holochain when asking it to install a hApp from the file system.

Adding a hApp to your scenario using the `Cargo.toml`:
```toml
[package.metadata.required-dna] # This can either be a single DNA or you can specify this multiple times as a list using [[package.metadata.required-dna]] 
name = "return_single_value" # The name to give the DNA that gets built
zomes = ["return_single_value"] # The name(s) of the zomes to include in the DNA, which must match the directory name in `./zomes`

[package.metadata.required-happ] # This can either be a single hApp or you can specify this multiple times as a list using [[package.metadata.required-happ]]
name = "return_single_value" # The name to give the hApp that gets built
dnas = ["return_single_value"] # The name(s) of the DNA to include in the hApp, which must match the name(s) given above.
```

If you need to debug this step, you can run `cargo build -p <your-scenario-crate>` and check the `dnas` and `happs` directories. 

### The Wind Tunnel Methodology

The Wind Tunnel framework is designed as a load testing tool. This means that the framework is designed to apply user-defined
load to a system and measure the system's response to that load. At a high-level there are two modes of operation. Either you run
the scenario and the system on the same machine and write the scenario to apply as much load as possible. Or you run the system in
a production-like environment and write the scenario to be distributed across many machines. The Wind Tunnel framework does not 
distinguish between these two modes of operation and will always behave the same way. It is up to you to write scenarios that are
appropriate for each mode of operation.

Load is applied to the system by agents. An agent is a single thread of execution that repeatedly applies the same behaviour to
the system. This is in the form of a function which is run repeatedly by Wind Tunnel. There are either many agents running in a 
single scenario to maximise load from a single machine, or many scenarios running in parallel that each have a single agent.
There is nothing stopping you from distributing the scenario and also running multiple agents but these are the suggested layouts
to design scenarios around.

In general a scenario consists of setup and teardown hooks, and an agent behaviour to apply load to the system. There are
global setup and teardown hooks that run once per scenario run. There are also agent setup and teardown hooks that run once
per agent during a scenario run. There are then one or more agent behaviours. For simple tests you just define a single behaviour
and all agents will behave the same way. For more complex tests you can define multiple behaviours and assign each agent to one
of them. This allows more complex test scenarios to be described where different agents take different actions and may interact 
with each other. For example, you might have some agents creating data and other agents just reading the data.

Wind Tunnel is not responsible for capturing information about your system. It can store the information that you collect and 
do some basic analysis on it. Alternatively, it can push metrics to InfluxDB. But it is up to you to collect the information that you need 
and to analyse it in detail. For example, the Wind Tunnel bindings for Holochain capture API response times on the app and admin 
interfaces and automatically reports this to Wind Tunnel but if you need to measure other things then you will need to write your own code to do that.

#### Stress Testing a Single Instance of Your Test System

In this first mode of operation you want to run the scenario and the system on the same machine. You should write the scenario to
apply as much load as possible to the system. That means keeping your agent behaviour hook as fast as possible. Preferably by
doing as much setup as possible in the agent setup hook and then just doing simple actions in the agent behaviour hook.

This kind of test is good for finding the limits of the system in a controlled environment. It can reveal things like high memory usage,
response times degrading over time and other bottlenecks in performance.

It may be useful to distribute this type of test. However, if it is written to maximise load then it only makes sense to distribute
it if the target system is also distributed in some way. With Holochain, for example, this wouldn't make sense because although Holochain
is distributed, it is not distributed in the sense of scaling performance for a single app.

#### Distributed Load Testing of Your System

In this second mode of operation you can still design and run the scenario on a single machine but that is just for development and
the value comes from running it in a distributed way. The intended configuration is to have many copies of the scenario binary distributed
across many machines. Each scenario binary will be configured to run a single agent. All the scenarios are configured to point at the same
test system. When testing Holochain, for example, Holochain is distributed first then a scenario binary is placed on each node with Holochain
and points at the local interface for Holochain.

Rather than looking to stress test the system in this mode, you are looking to measure the system's response to a realistic load. This is not
understood by Wind Tunnel but you are permitted to block the agent behaviour hook to slow down the load the Wind Tunnel runner will apply. 
This allows you to be quite creative when designing your load pattern. For example, you could define a common agent behaviour function then 
create multiple agent behaviour hooks within your scenario that are use the common function at different rates. This would simulate varied 
behaviour by different agents.

### Writing Scenarios for Holochain

> [!NOTE]
> Writing scenarios requires some knowledge of `wind-tunnel`'s methodology. That is assumed knowledge for this section!

Writing a Wind Tunnel scenario is relatively straight forward. The complexity is mostly in the measurement and analysis of the system
once the scenario is running. To begin, you need a Rust project with a single binary target.

`cargo new --bin --edition 2021 my_scenario`

You will probably need more dependencies at some point, but the minimum to get started are the `holochain_wind_tunnel_runner` and 
`holochain_types` crates.

```bash
cargo add holochain_wind_tunnel_runner
cargo add holochain_types
```

If this scenario is being written inside this repository then there are some extra setup steps. Please see the [project layout docs](#navigating-the-project).

Add the following imports to the top of your `main.rs`:

```rust
use holochain_types::prelude::ExternIO;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
```

Then replace your `main` function with the following:

```rust
fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<HolochainRunnerContext, HolochainAgentContext>::new_with_init(
        env!("CARGO_PKG_NAME"),
    )
    .with_default_duration_s(60)
    .use_agent_behaviour(agent_behaviour);

    run(builder)?;

    Ok(())
}
```

This is the basic structure of a Wind Tunnel scenario. The `ScenarioDefinitionBuilder` is used to define the scenario. It includes
a CLI which will allow you to override some of the defaults that are set in your code. Using the builder you can configure your hooks
which are just Rust functions that take a context and return a `WindTunnelResult`. 

The `run` function is then called with the builder. At that point the Wind Tunnel runner takes over and configures, then runs your scenario.

Before you can run this, you'll need to provide the agent behaviour hook. Add the following to your `main.rs`:

```rust
fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    println!("Hello from, {}", ctx.agent_name());
    std::thread::sleep(std::time::Duration::from_secs(1));
    Ok(())
}
```

This is just an example hook and you will want to replace it once you have got your scenario running. Note the `AgentContext` that is provided
to the hook. This is created per-agent and gives you access to the agent's ID and the runner's context. Both the agent and the runner context are
used for sharing configuration between the runner and your hooks, and state between your hooks.

Your scenario should now be runnable. Try running it with

```bash
cargo run -- --duration 10
```

You should see the print messages from the agent behaviour hook. If so, you are ready to start writing your scenario. To get started,
you are recommended to take a look at documentation for the `holochain_wind_tunnel_runner` crate. This has common code to use in your
your scenarios and example of how to use them. This will help you get started much more quickly than starting from scratch. There is
also a tips section below which you might find helpful as you run into questions.


### Writing Scenarios for Kitsune

> [!NOTE]
> Writing scenarios requires some knowledge of `wind-tunnel`'s methodology as well as an overview of how Kitsune works. That is assumed knowledge for this section!

Writing a Kitsune Wind Tunnel scenario is relatively straight forward. The Kitsune client defines three common functions for the developer. A chatter can be `create`d, it can `join_chatter_network` and it can `say` a list of messages. As long as a chatter has not joined the network, it won't receive messages from other peers and will also not send messages to them. Once joined, it starts receiving and sending messages it has said. It will also receive messages that were sent before it joined the network.

For communication among peers to work, a bootstrap server must be running that enables peers to discover each other, and a signal server is required for establishing direct WebRTC connections. See [Kitsune Tests](#kitsune-tests).

The only Wind Tunnel specific dependency you will need is `kitsune_wind_tunnel_runner`.

```bash
cargo add kitsune_wind_tunnel_runner
```

If this scenario is being written inside this repository then there are some extra setup steps. Please see the [project layout docs](#navigating-the-project).

Add the following imports to the top of your `main.rs`:

```rust
use kitsune_wind_tunnel_runner::prelude::*;
```

Then replace your `main` function with the following:

```rust
fn main() -> WindTunnelResult<()> {
    let builder =
        KitsuneScenarioDefinitionBuilder::<KitsuneRunnerContext, KitsuneAgentContext>::new_with_init(
            "scenario_name",
        )?.into_std()
        .use_agent_behaviour(agent_behavior);

    run(builder)?;

    Ok(())
}
```

This is the basic structure of a Kitsune Wind Tunnel scenario. The `ScenarioDefinitionBuilder` is used to define the scenario. It includes
a CLI which will allow you to override some of the defaults that are set in your code. Using the builder you can configure your hooks
which are just Rust functions that take a context and return a `WindTunnelResult`. 

The `run` function is then called with the builder. At that point the Wind Tunnel runner takes over and configures, then runs your scenario.

Before you can run this, you'll need to provide the agent behaviour hook. Add the following to your `main.rs`:

```rust
fn agent_behaviour(
    ctx: &mut AgentContext<KitsuneRunnerContext, KitsuneAgentContext>,
) -> HookResult {
    println!("Hello from, {}", ctx.agent_name());
    std::thread::sleep(std::time::Duration::from_secs(1));
    Ok(())
}
```

This is just an example hook and you will want to replace it once you have got your scenario running. Note the `KitsuneAgentContext` that is provided
to the hook. This is created per-agent and gives you access to the agent's ID and the runner's context as well as the chatter ID which is specific to
Kitsune. Both the agent and the runner context are used for sharing configuration between the runner and your hooks, and state between your hooks.

Your scenario should now be runnable. Try running it with

```bash
cargo run -- --bootstrap-server-url http://127.0.0.1:30000 --signal-server-url ws://127.0.0.1:30000 --duration 10
```

You should see the print messages from the agent behaviour hook. If so, you are ready to start writing your scenario.

### Tips for Writing Scenarios

#### Run async code in your agent behaviour

The behaviour hooks are synchronous but the Holochain client is asynchronous. The ability to run async code in your hooks is exposed
through the `AgentContext` and `RunnerContext`.

```rust
fn agent_behaviour(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
    ctx.runner_context().executor().execute_in_place(async {
        // Do something async here
    })?;

    Ok(())
}
```

#### Record custom metrics

This is useful for scenarios that need to measure things that don't happen through the instrumented client that is talking to the system under test.

```rust
fn agent_behaviour(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
    let metric = ReportMetric::new("my_custom_metric")
        .with_field("value", 1);
    ctx.runner_context().reporter().clone().add_custom(metric);
    
    Ok(())
}
```

The metric will appear in InfluxDB as `wt.custom.my_custom_metric` with a field `value` set to `1`.

### Running scenarios locally with Nix

When developing your scenarios, you can disable anything that requires running infrastructure, other than the target system. However, once you
are ready to run your scenario to get results, you will need a few extra steps.

#### Running InfluxDB

InfluxDB is used to store the metrics that Wind Tunnel collects. You can run it locally from inside a Nix shell launched with `nix develop`:

```bash
influxd
```

This terminal will then be occupied running InfluxDB. Start another terminal where you can configure the database and create a user, again from inside the Nix shell:

```bash
configure_influx
```

This will do a one-time setup for InfluxDB and also configure your shell environment to use it. Next time you start a new terminal you will need to run `use_influx` instead.

You can now navigate to the InfluxDB [dashboard](http://localhost:8087) and log in with `windtunnel`/`windtunnel`. The variables and dashboards you need will already be set up,
so you can now run your scenario and the metrics will be pushed to InfluxDB.

#### Running Telegraf

Telegraf is used for collecting host metrics and writing them to disk. This is not required locally, but if you would like to run it, then you can do so from inside the Nix shell.
Currently, `telegraf` is configured to collect the following metrics:

- CPU stats
- Disk stats and usage
- Kernel Info
- Memory and Swap stats
- Network stats
- Processes
- System stats

To run the `telegraf` agent to collect host metrics while running scenarios enter the Nix shell and run

```sh
start_host_metrics_telegraf
```

#### Running Holochain

For a zero-config and quick way to run Holochain, you can use the following command:

```bash
hc s clean && echo "1234" | hc s --piped create && echo "1234" | RUST_LOG=warn hc s --piped -f 8888 run
```

For more advanced scenarios or for distributed tests, this is not appropriate!


To run Holochain with metrics enabled, the `HOLOCHAIN_INFLUXIVE_FILE` environment variable must be set beforehand to a valid path within `WT_METRICS_DIR` (set by the Nix shell).
For example:
```bash
export HOLOCHAIN_INFLUXIVE_FILE=$WT_METRICS_DIR/holochain.influx
```

#### Running scenarios

Each scenario is expected to provide a README.md with at least:

- A description of the scenario and what it is testing for.
- A suggested command or commands to run the scenario, with justification for the configuration used.

For example, see the [zome_call_single_value](https://github.com/holochain/wind-tunnel/blob/main/scenarios/zome_call_single_value/README.md) scenario.

As well as the command you use to run the scenario, you will need to select an appropriate reporter. Run the scenario with the `--help` flag to see the available options.
For local development, the default `in-memory` reporter will do.
If you have influx running and only want scenario metrics, then you can use the `influx-client` option.
If you have set up Holochain or host metrics then you can use the `influx-file` option and then import all metrics in the next step.

#### Importing Metrics

Once you've finished running a scenario, you can collect host, Holochain and scenario metrics with:

```sh
nix run .#local-upload-metrics
```

At this point the metrics will be uploaded to InfluxDB, and you will be able to view the metrics in the InfluxDB dashboards by `run_id`.

Running this Nix command will also clean up the current metrics from disk, so you are immediately ready to run the next scenario.

> [!Warning]
> The metrics must be imported after each scenario run since they are associated only with the latest scenario run.
> [!Warning]
> If Holochain ran with metrics enabled, it must be restarted after each scenario run since its output file is deleted after importing.
> [!Warning]
> If host metrics were enabled with Telegraf, it must be restarted after each scenario run since its output file is deleted after importing.

### Developer guide (for wind-tunnel)

There is a Nix environment provided, and it is recommended that you use its shell for development:

```bash
nix develop
```

Decide what type of test you are writing and pick one of the next two sections.
Then you can move to writing and running the scenario.

#### Standard Wind Tunnel tests

For standard Wind Tunnel tests - start a sandbox for testing:

```bash
hc s clean && echo "1234" | hc s --piped create && echo "1234" | hc s --piped -f 8888 run
```

It is recommended to stop and start this sandbox conductor between test runs, because getting Holochain back to a clean state
through its API is not yet implemented.

You can then start a second terminal and run one of the scenarios in the `scenarios` directory:

```bash
RUST_LOG=info cargo run -p zome_call_single_value -- --duration 60 -c ws://localhost:8888
```

#### Running Wind Tunnel Scenarios with Nomad

##### Running Locally

You can easily test the Wind Tunnel scenarios with
[Nomad](https://www.nomadproject.io) by running them locally. This requires
running a Nomad agent locally as both a client and a server.

First, enter the Nix `devShell` with `nix develop` to make sure you have all the packages install.
Alternatively, [install Nomad](https://developer.hashicorp.com/nomad/install) and Holochain locally
so that both `nomad` and `hc` are in your `PATH`.

Once Nomad is installed, run the agent in `dev` mode to spin up both a server and client, do this with:

```bash
nomad agent -dev
```

Now navigate to <http://localhost:4646/ui> to view the Nomad dashboard.

Next, in a new terminal window, build the scenario you want to run with:

```bash
nix build .#app_install
```

Replace `app_install` with the name of the scenario that you want to run.

Once the scenario is built you can run the Nomad job with:

```bash
nomad job run -address=http://localhost:4646 -var scenario-url=result/bin/app_install -var reporter=in-memory nomad/jobs/app_install_minimal.nomad.hcl
```

All the jobs are in the `nomad/jobs` directory, so you can replace `app_install_minimal` with the name of the job you want to run.

- `-address` sets Nomad to talk to the locally running instance and not the dedicated Wind Tunnel cluster one.
- `-var scenario-url=...` provides the path to the scenario binary that you built in the previous step.
- `-var reporter=in-memory` sets the reporter type to print to `stdout` instead of writing an InfluxDB metrics file.

> [!Warning]
> When running locally as in this guide, the `reporter` must be set to `in-memory` and the `scenario-url` must be a
> local path due to the way Nomad handles downloading. To get around this limitation you must disable file system
> isolation, see <https://developer.hashicorp.com/nomad/docs/configuration/client#disable_filesystem_isolation>.

You can also override existing and omitted variables with the `-var` flag. For example, to set the duration (in seconds) use:

```bash
nomad job run -address=http://localhost:4646 -var scenario-url=result/bin/app_install -var reporter=in-memory -var duration=300 nomad/jobs/app_install_minimal.nomad.hcl
```

> [!Note]
> Make sure the `var` options are after the `var-file` option otherwise the values in the file will take precedence.

Then, navigate to <http://localhost:4646/ui/jobs/run_scenario@default> where you should see one allocation,
which is the Nomad name for an instance of the job. You can view the logs of the tasks to see the results.
The allocation should be marked as "complete" after the duration specified.

Once you've finished testing you can kill the Nomad agent with `^C` in the first terminal running the agent.

##### Wind Tunnel Nomad Cluster

Wind Tunnel has a dedicated Nomad cluster for running scenarios.

This cluster can be accessed at <https://nomad-server-01.holochain.org:4646/ui>.
A token is required to view the details of the cluster, the shared admin "bootstrap" token can be
found in the Holochain shared vault of the password manager under `Nomad Server Bootstrap Token`.

Enter the token (or use auto-fill) to sign in at <https://nomad-server-01.holochain.org:4646/ui/settings/tokens>.

You can now view any recent or running jobs at <https://nomad-server-01.holochain.org:4646/ui/jobs>.

###### Running Scenarios from the Command-Line

> [!Note]
> Running scenarios on the remote cluster from the command-line requires quite a few steps including
> storing the binary on a public file share. For that reason it is recommended to instead use the
> [Nomad workflow](https://github.com/holochain/wind-tunnel/actions/workflows/nomad.yaml) which
> takes care of some of these steps for you.

To run a Wind Tunnel scenario on the Nomad cluster from the command-line, first enter the Nix `devShell`
with `nix develop` or [install Nomad](https://developer.hashicorp.com/nomad/install) locally.

You also need to set the `NOMAD_ADDR` environment variable to `https://nomad-server-01.holochain.org:4646`
and `NOMAD_CACERT` to `./nomad/server-ca-cert.pem`, which are both set by the Nix `devShell`.

The final environment variable that needs to be set and is **not** set by the `devShell` is `NOMAD_TOKEN`
which needs to be set to a token with the correct permissions, for now it is fine to just use the admin
token found in the Holochain shared vault of the password manager under `Nomad Server Bootstrap Token`.

Once Nomad is installed, bundle the scenario you want to run with Nix so that it can run on other machines.

Run:

```bash
nix bundle .#packages.x86_64-linux.app_install
```

Replace `app_install` with the name of the scenario that you want to run.
This will build and bundle the scenario to run on any `x86_64-linux` machine and does not require Nix to run.
The bundled output will be in your `/nix/store/` with a symlink to it in your local dir with an `-arx` postfix,
to make it easier to find the bundle later it is recommended to copy it somewhere. It is also best to remove
the `-arx` postfix now so we don't forget later.

```bash
cp ./app_install-arx ./app_install
```

You now need to upload the scenario bundle to somewhere public so that the Nomad client can download it.
This could be a GitHub release, a public file sharing services, or some other means, as long as it's publicly
accessible.

> [!Note]
> Unlike when running locally in the section above, we cannot just pass a path because the path needs to be
> accessible to the client and Nomad doesn't have native support for uploading artefacts.

Now that the bundle is publicly available you can run the scenario with the following:

```bash
nomad job run -var-file=nomad/var_files/app_install_minimal.vars -var scenario-url=http://{some-url} nomad/run_scenario.nomad.hcl
```

- `-var-file` should point to the var file, in `nomad/var_files`, of the scenario you want to run.
- `-var scenario-url=...` provides the URL to the scenario binary that you uploaded in the previous step.

You can also override existing and omitted variables with the `-var` flag. For example, to set the duration
(in seconds) or to set the reporter to print to `stdout`.

```bash
nomad job run -var-file=nomad/var_files/app_install_minimal.vars -var scenario-url=http://{some-url} -var reporter=in-memory -var duration=300 nomad/run_scenario.nomad.hcl
```

> [!Note]
> Make sure the `var` options are after the `var-file` option otherwise the values in the file will take precedence.

Then, navigate to <https://nomad-server-01.holochain.org:4646/ui/jobs/run_scenario@default> where you
should see the allocation, which is the Nomad name for an instance of the job. You can view the logs
of the tasks to see the results. The allocation should be marked as "complete" after the duration specified.

You can now get the run ID from the `stdout` of the `run_scenario` task in the Nomad web UI and, if the `reporter`
was set to `influx-file` (the default value) then you can use that ID to view the results on the corresponding
InfluxDB dashboard, the dashboards can be found at <https://ifdb.holochain.org/orgs/37472a94dbe3e7c1/dashboards-list>,
the credentials of which can be found in the Holochain shared vault of the password manager.

###### Running Scenarios with the CI

There is a [dedicated GitHub workflow](https://github.com/holochain/wind-tunnel/actions/workflows/nomad.yaml)
for bundling all the scenarios designed to run with Nomad, uploading them as GitHub artifacts, and then
running them on available Nomad clients specifically available for testing. The metrics from the runs
are also uploaded to the InfluxDB instance. This is the recommended way to run the Wind Tunnel scenarios
with Nomad.

To run it, simply navigate to <https://github.com/holochain/wind-tunnel/actions/workflows/nomad.yaml>, select
`Run workflow` on the right, and select the branch that you want to test. If you only want to test a
sub-selection of the scenarios then simply comment-out or remove the scenarios that you want to exclude
from the matrix in [the workflow file](.github/workflows/nomad.yaml), push your changes and make sure to
select the correct branch.

> [!Warning]
> Currently, the `Wait for free nodes` step will wait indefinitely if there are never enough free nodes
> which will also block other jobs from running.

#### Kitsune tests

For Kitsune Wind Tunnel tests, start a bootstrap and signal server:
```bash
kitsune2-bootstrap-srv --listen 127.0.0.1:30000
```

This starts the two servers on the provided address. If for some reason the port 30000 is used on your system, you can specify a different port or omit the `--listen` option altogether to let the command choose a free port.

You can then start a second terminal and run one of the scenarios in the `scenarios` directory that start with `kitsune_`:

```bash
RUST_LOG=info cargo run -p kitsune_continuous_flow -- --bootstrap-server-url http://127.0.0.1:30000 --signal-server-url ws://127.0.0.1:30000  --duration 20 --agents 2
```

If your bootstrap and signal servers run under a different port, adapt the command accordingly. The scenario creates 2 peer and runs for 20 seconds.

### Published crates

Framework crates:

- [![crates.io](https://img.shields.io/crates/v/wind_tunnel_core)](https://crates.io/crates/wind_tunnel_core) Core functionality for use by other Wind Tunnel crates - [wind_tunnel_core](https://github.com/holochain/wind-tunnel/tree/main/framework/core)
- [![crates.io](https://img.shields.io/crates/v/wind_tunnel_instruments)](https://crates.io/crates/wind_tunnel_instruments) Instruments for measuring performance with Wind Tunnel - [wind_tunnel_instruments](https://github.com/holochain/wind-tunnel/tree/main/framework/instruments)
- [![crates.io](https://img.shields.io/crates/v/wind_tunnel_instruments_derive)](https://crates.io/crates/wind_tunnel_instruments_derive) Derive macros for the wind_tunnel_instruments crate - [wind_tunnel_instruments_derive](https://github.com/holochain/wind-tunnel/tree/main/framework/instruments_derive)
- [![crates.io](https://img.shields.io/crates/v/wind_tunnel_runner)](https://crates.io/crates/wind_tunnel_runner) The Wind Tunnel runner - [wind_tunnel_runner](https://github.com/holochain/wind-tunnel/tree/main/framework/runner)

Bindings crates for Holochain:

- [![crates.io](https://img.shields.io/crates/v/holochain_client_instrumented)](https://crates.io/crates/holochain_client_instrumented) An instrumented wrapper around the holochain_client - [holochain_client_instrumented](https://github.com/holochain/wind-tunnel/tree/main/bindings/client)
- [![crates.io](https://img.shields.io/crates/v/holochain_wind_tunnel_runner)](https://crates.io/crates/holochain_wind_tunnel_runner) Customises the wind_tunnel_runner for Holochain testing - [holochain_wind_tunnel_runner](https://github.com/holochain/wind-tunnel/tree/main/bindings/runner)

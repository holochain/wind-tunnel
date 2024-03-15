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

The `bindings`, found in the `./bindings`, customise the `wind-tunnel` framework to Holochain. They are what you would be
consuming when using `wind-tunnel` to test Holochain.

The bindings contains the following crates:
- `holochain_client_instrumented`: A wrapper around the [`holochain_client`](https://crates.io/crates/holochain_client) that uses `instruments` and `instruments_derive`
  to instrument the client. It exports an API that is nearly identical to the `holochain_client` crate, except that when constructing client connections
  you need to provide a reporter which it can write results to.
- `holochain_wind_tunnel_runner`: This is a wrapper around the `wind_tunnel_runner` crate that provides Holochain specific code to be used with the `wind_tunnel_runner`.
  The `wind_tunnel_runner` is re-exported, so you should just use this crate as your runner when creating tests for Holochain.

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
once the scenario is running. To begin, you need a Rust project that with a single binary target.

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

The `run` function is then called with the builder. At that point the Wind Tunnel runner takes over and configures then runs your scenario.

Before you can run this, you'll need to provide the agent behaviour hook. Add the following to your `main.rs`:

```rust
fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    println!("Hello from, {}", ctx.agent_id());
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

### Running scenarios locally

When developing your scenarios you can disable anything that requires running infrastructure, other than the target system. However, once you
are ready to run your scenario to get results you will need a few extra steps.

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

This is used for pushing system metrics to InfluxDB. This is not required locally but if you would like to run it then you can do so from inside the Nix shell:

```bash
use_influx
start_telegraf
```

#### Running Holochain

For a zero-config and quick way to run Holochain, you can use the following command:

```bash
hc s clean && echo "1234" | hc s --piped create && echo "1234" | hc s --piped -f 8888 run
```

For more advanced scenarios or for distributed tests, this is not appropriate!

#### Running scenarios

Each scenario is expected to provide a README.md with at least:
- A description of the scenario and what it is testing for.
- A suggested command or commands to run the scenario, with justification for the configuration used.

For example, see the [zome_call_single_value](https://github.com/holochain/wind-tunnel/blob/main/scenarios/zome_call_single_value/README.md) scenario.

### Developer guide (for wind-tunnel)

There is a Nix environment provided, and it is recommended that you use its shell for development:

```bash
nix develop
```

Start a sandbox for testing:

```bash
hc s clean && echo "1234" | hc s --piped create && echo "1234" | hc s --piped -f 8888 run
```

It is recommended to stop and start this sandbox conductor between test runs because getting Holochain back to a clean 
through its API is not yet implemented.

You can then start a second terminal and run one of the scenarios in the `scenarios` directory:

```bash
RUST_LOG=info cargo run -p zome_call_single_value -- --duration 60 -c ws://localhost:8888
```

### Published crates

Framework crates:
- [![crates.io](https://img.shields.io/crates/v/wind_tunnel_instruments)](https://crates.io/crates/wind_tunnel_instruments) Instruments for measuring performance with Wind Tunnel - [wind_tunnel_instruments](https://github.com/holochain/wind-tunnel/tree/main/framework/instruments)
- [![crates.io](https://img.shields.io/crates/v/wind_tunnel_instruments_derive)](https://crates.io/crates/wind_tunnel_instruments_derive) Derive macros for the wind_tunnel_instruments crate - [wind_tunnel_instruments_derive](https://github.com/holochain/wind-tunnel/tree/main/framework/instruments_derive)
- [![crates.io](https://img.shields.io/crates/v/wind_tunnel_runner)](https://crates.io/crates/wind_tunnel_runner) The Wind Tunnel runner - [wind_tunnel_runner](https://github.com/holochain/wind-tunnel/tree/main/framework/runner)

Bindings crates for Holochain:
- [![crates.io](https://img.shields.io/crates/v/holochain_client_instrumented)](https://crates.io/crates/holochain_client_instrumented) An instrumented wrapper around the holochain_client - [holochain_client_instrumented](https://github.com/holochain/wind-tunnel/tree/main/bindings/client)
- [![crates.io](https://img.shields.io/crates/v/holochain_wind_tunnel_runner)](https://crates.io/crates/holochain_wind_tunnel_runner) Customises the wind_tunnel_runner for Holochain testing - [holochain_wind_tunnel_runner](https://github.com/holochain/wind-tunnel/tree/main/bindings/runner)

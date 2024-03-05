# Wind Tunnel

Performance testing for Holochain, modelled as load tests. The name is a reference to aerodynamics testing and is a good
way to refer to this project but the language does not extend to the code. 

### Navigating the project

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

There is more information about how to create scenarios in a separate section.

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

### Developer guide (for wind-tunnel)

There is a Nix environment provided and it is recommended that you use its shell for development:

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

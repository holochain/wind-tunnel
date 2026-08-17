# Nomad Jobs

## Create new jobs

In order to define new jobs, you just need to create a new vars file in the `./vars` directory with a JSON file with the
name of the job you want to create. For example, if you want to create a job for the `app_install` scenario, you can
create a file named `app_install.json` in the `nomad/vars` directory.

### Vars file syntax

A simple Example:

```json
{
  "job_name": "app_install_minimal_demo",
  "scenario_name": "app_install",
  "duration": 300,
  "assignments": [
    {
      "behaviour": "large",
      "nodes": 2,
      "agents": 5
    }
  ]
}
```

An example with environment variables:

```json
{
  "job_name": "write_get_agent_activity_volatile_demo",
  "scenario_name": "write_get_agent_activity_volatile",
  "duration": 900,
  "assignments": [
    {
      "behaviour": "write",
      "nodes": 2
    },
    {
      "behaviour": "get_agent_activity_volatile",
      "nodes": 8
    }
  ],
  "env": {
    "CONDUCTOR_ON_MIN_S": "10",
    "CONDUCTOR_ON_MAX_S": "30",
    "CONDUCTOR_OFF_MIN_S": "2",
    "CONDUCTOR_OFF_MAX_S": "10"
  }
}
```

The following variables are available:

- `description`: A human-readable description of what the job does. (_optional_, not used at runtime — serves as documentation for the vars file)
- `job_name`: The name of the Nomad job where all runs of this scenario will be grouped under. (**required**)
- `scenario_name`: The name of the scenario you want to run. (**required**)
- `duration`: The duration of the scenario in seconds. (**required**)
- `assignments`: A list of assignments to apply to the scenario. (_optional_, defaults to `[{"behaviour": "default"}]`)
  - `behaviour`: The behaviour to apply to the nodes in this assignment. (_optional_, defaults to `"default"`)
  - `nodes`: The number of nodes to run with this behaviour. (_optional_, defaults to `1`)
  - `agents`: The number of agents to run on each node with this behaviour. (_optional_, defaults to `1`)
- `connection_string`: The connection string to the Holochain conductor. (_optional_, defaults to `ws://localhost:8888`)
- `run_id`: The ID of the run to distinguish it from other runs. (_optional_, defaults to `null`)
- `reporter`: The reporter type to use. (_optional_, defaults to `influx-file`)
- `env`: An object of arbitrary key-value pairs to set as environment variables for the scenario run. (_optional_, defaults to `{}`)
  Each key-value pair is injected into the Nomad task's `env` block. This can be used to configure scenario-specific
  behavior. For example, `MIN_AGENTS` controls the minimum number of agents each agent will wait for before running.
  See individual scenario documentation for available environment variables.
- `runtime`: Selects which job template renders the vars file. (_optional_, defaults to `holochain`)
  The value must match a template `nomad/<runtime>_scenario.tpl.hcl`, currently either `holochain` (default) or
  `peerkit`.

## Generate Nomad Jobs

Once you have created the vars file, you can generate the Nomad job files by running the following commands:

```bash
nix run .#generate-nomad-jobs --job-variant-path nomad/job-variants/demo
nix run .#generate-nomad-jobs --job-variant-path nomad/job-variants/canonical
nix run .#generate-nomad-jobs --job-variant-path nomad/job-variants/canonical-scaled
```

This will generate the nomad job files in the `nomad/job-variants/demo`, `nomad/job-variants/canonical`, and `nomad/job-variants/canonical-scaled` directories.
The job files will be named after the scenario name, with the `.nomad.hcl` extension.

Mind that in order to generate the jobs, you need to have `gomplate` and `jq` installed. You can use the ones provided
by the nix shell in this repository or download the latest `gomplate` from
the [gomplate releases page](https://github.com/hairyhenderson/gomplate/releases).

## Jobs template

By default, jobs are generated from the Holochain template, located in `nomad/holochain_scenario.tpl.hcl`. This
template uses the variables defined in the vars file to generate the Nomad job file. Vars files can select a
different template by setting the `runtime` key (see above).

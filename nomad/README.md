# Nomad Jobs

## Create new jobs

In order to define new jobs, you just need to create a new vars file in the `./vars` directory with a JSON file with the name of the job you want to create. For example, if you want to create a job for the `app_install` scenario, you can create a file named `app_install.json` in the `nomad/vars` directory.

### Vars file syntax

A simple Example:

```json
{
  "scenario_name": "app_install",
  "behaviours": [
    "large"
  ]
}
```

The following variables are available:

- `scenario_name`: The name of the scenario you want to run. (**required**)
- `duration`: The duration of the scenario in seconds. (**required**)
- `behaviours`: A list of behaviours to apply to the scenario. (_optional_, defaults to `[""]`)
- `connection_string`: The connection string to the Holochain conductor. (_optional_, defaults to `ws://localhost:8888`)
- `run_id`: The ID of the run to distinguish it from other runs. (_optional_, defaults to `null`)
- `agents_per_node`: The number of agents per node. (_optional_, defaults to `1`)
- `min_agents`: The minimum number of agents to run the scenario with. (_optional_, defaults to `2`)
- `reporter`: The reporter type to use. (_optional_, defaults to `influx-file`)

## Generate Nomad Jobs

Once you have created the vars file, you can generate the Nomad job file by running the following command:

```bash
./nomad/scripts/generate_jobs.sh
```

This will generate the nomad job files in the `nomad/jobs` directory. The job files will be named after the scenario name, with the `.nomad.hcl` extension.

Mind that in order to generate the jobs, you need to have `gomplate` installed. You can use the one provided by nix shell in this repository or download the latest version from the [gomplate releases page](https://github.com/hairyhenderson/gomplate/releases).

## Jobs template

Currently, all the jobs are generated from the same template, which is located in `nomad/run_scenario.tpl.hcl`. This template uses the variables defined in the vars file to generate the Nomad job file.

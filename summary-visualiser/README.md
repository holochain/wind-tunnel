# Wind Tunnel Summary Visualiser

A tool to take a summary report from a Wind Tunnel scenario run, in JSON format, and turn it into a pretty HTML report with graphs, so you can grok the information more deeply and quickly. It uses [`gomplate`](https://gomplate.ca/) in a bash script to do its work.

## Prerequisites

Either install the `gomplate` templating tool, or run `nix develop` in the repo root to get this tool.

## Usage

The command takes input JSON (either as a filename or from stdin) and outputs HTML to stdout.

You can use this tool on a recent artifact containing all scenarios by running:

```bash
summary-visualiser/generate.sh summary-visualiser/test_data/all.json > out.html
```

and opening `out.html` in your browser. It'll contain a `<section class="scenario scenario-foo">` element for every scenario in your JSON. If your JSON contains scenarios for which there are no templates yet, it'll display a warning for each of them.

### With ideal sample data

There are some individual scenario sample files in `summary-visualiser/test_data/` that you can use for testing too. They're likely to have more complete sets of data than that what you'll find in `all.json`, although their metrics may not be as realistic. However, they're just bare objects, and this tool expects the input JSON to be an array of objects. You can wrap the individual objects in an array like this (make sure you have `jq` installed, or run `nix develop` to get it).

```bash
cat summariser/test_data/3_summary_outputs/dht_sync_lag-3a1e33ccf661bd873966c539d4d227e703e1496fb54bb999f7be30a3dd493e51.json | jq '[.]' | summary-visualiser/generate.sh > dht_sync_lag.html
```

## Provided template assets

This tool is meant to be versatile. Currently it's hard-coded to look for a file called `templates/page.html.tmpl` to use as its main page template, but it would be an easy exercise to modify it to take any input data.

If you do create your own page template, you have a few assets to get you started:

* `assets/wind_tunnel.js`: Required for rendering trend graphs. Uses d3.js.
* `assets/wind_tunnel.css`: Some generic CSS to produce a good starting point for styling scenarios. Not too forceful, so you should be able to override most or all of these styles in your own pages without trouble.
* `templates/helpers/scenarios_loop.html.tmpl`: Loop through all scenarios in the input JSON, look for a `templates/scenarios/<scenario_name>.html.tmpl` file, and pass the scenario data to that file for rendering.
* `templates/helpers/scenarios_title.html.tmpl`: Generate text for an HTML `<title>` element from all the scenarios in the input JSON. Truncates to three scenarios and counts the rest.
* `templates/helpers/scenarios_description.html.tmpl`: Generate text for an HTML `<meta name="description"/>` element from all the scenarios. Lists all scenarios.

## Maintenance

When you create a scenario, you need to:

1. Add a file called `<scenario_name>.html.tmpl` to the `templates/scenarios` folder.
2. Populate that file with an HTML template that renders the scenario's data (you can take a look at that folder's README or other scenario templates to get a sense of how to build one).
3. Replace `test_data/all.json` with a summary output artifact from a recent Nomad run that contains your new scenario.
4. Add the line `smoke_test_scenario "<scenario_name>"` to `test.sh`.
5. Run `./test.sh` and look for errors. (Note that, when you commit the file, a pre-commit check will check for errors and invalid HTML.)

When you modify a scenario, you need to:

1. Modify `templates/scenarios/<scenario_name>.html.tmpl` to match the scenario's changed summary JSON structure.
2. Replace `test_data/all.json` with a summary output artifact from a recent Nomad run that contains the modifications to the scenario.
3. Run `./test.sh` and look for errors.

When you delete a scenario, you need to remove its line from `./test.sh`. It'd also be good to tidy up by removing the scenario template and updating `test_data.all.json`.

## Nomad workflow integration

The output HTML is published as part of the GitHub Pages for this repository at [https://holochain.github.io/wind-tunnel/](https://holochain.github.io/wind-tunnel/) after each run of the `Run performance tests on Nomad cluster` workflow.

In the `.github/workflows/nomad.yaml` file, the workflow step that does this is:

```yaml
- name: Generate summary visualiser
  run: |
    mkdir -p ./nomad-summary-visualiser
    nix run .#generate-summary-visualiser ./summariser-report-*.json ./nomad-summary-visualiser/index.html

- name: Push index.html to GitHub Pages
  uses: peaceiris/actions-gh-pages@v3
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    publish_dir: ./nomad-summary-visualiser # The directory containing index.html
    publish_branch: gh-pages
```

If you want you can specify the commit hash to checkout before running the tests by providing the `commit-hash` input to the workflow.

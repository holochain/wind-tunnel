# Wind Tunnel Summary Visualiser

A tool to take a summary report from a Wind Tunnel scenario run, in JSON format, and turn it into a pretty HTML report with graphs, so you can grok the information more deeply and quickly. It uses [`gomplate`](https://gomplate.ca/) in a bash script to do its work.

## Prerequisites

Either install the `gomplate` templating tool, or run `nix develop` in the repo root to get this tool.

## Usage

The command takes input JSON (either as a filename or from stdin) and outputs to stdout.

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

## Nomad Workflow Integration

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

If you want you can specify the commit hash to checkout before running the tests by providing the `commit_hash` input to the workflow.

# Wind Tunnel Summary Visualiser

A tool to take a summary report from a Wind Tunnel scenario run, in JSON format, and turn it into a pretty HTML report with graphs, so you can grok the information more deeply and quickly. It uses [`gomplate`](https://gomplate.ca/) in a bash script to do its work.

## Prerequisites

Either install `jq` and `gomplate`, or run `nix develop` in the repo root to get those tools.

## Usage

The command takes input JSON (either as a filename or from stdin) and outputs to stdout.

```bash
summary-visualiser/generate.sh foo.json > out.html
```

### With sample data

This tool (or rather, its template) expects the input JSON to be an array of objects. Use the sample files in `summary-visualiser/test_data/` (rather than the ones in `summariser/test_data/3_summary_outputs/`, which are just bare objects):

```bash
cd summary-visualiser
./generate.sh test_data/dht_sync_lag.json > dht_sync_lag.html
```

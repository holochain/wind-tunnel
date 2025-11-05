# Wind Tunnel Summary Visualiser

A tool to take a summary report from a Wind Tunnel scenario run, in JSON format, and turn it into a pretty HTML report with graphs, so you can grok the information more deeply and quickly. It uses [`gomplate`](https://gomplate.ca/) in a bash script to do its work.

## Prerequisites

Either install `jq` and `gomplate`, or run `nix develop` in the repo root to get those tools.

## Usage

The command takes input JSON (either as a filename or from stdin) and outputs to stdout.

```bash
summary-visualiser/generate.sh summariser/test_data/3_summary_outputs/dht_sync_lag-3a1e33ccf661bd873966c539d4d227e703e1496fb54bb999f7be30a3dd493e51.json > out.html
```

That's all you need to know!

# conductor_startup

## Description

This scenario measures conductor startup as installed cells move from disabled
to enabled. It supports the Holochain 0.7.0 lazy-loading proof-out: an
installed but disabled cell should have little effect on startup time. The
workload resembles Moss deployments that maintain many independent networks.

## How it works

Each scenario process runs exactly one agent and manages its own conductor. It
performs the following sequence:

1. Starts an empty conductor and records its startup time as the `initial`
   phase.
2. Installs `WT_CELL_COUNT` copies of the callback hApp. Each app has a unique
   network seed and remains disabled after installation.
3. Restarts the conductor with every app still disabled and records its startup
   time as the `post_install` phase.
4. Runs the `default` behaviour repeatedly. Each iteration enables exactly one
   app and records the enable time. There is no fixed delay between iterations.
5. When `WT_RESTART_INTERVAL` is greater than zero, restarts the conductor after
   that many successful enables and records its startup time as the `periodic`
   phase. The interval is a number of enabled apps, not a time interval.
6. Stops after enabling the final app. It does not restart the conductor after
   that final enable.

This produces startup measurements after fixed-size batches of apps become
enabled, which can be compared with the empty and fully disabled baselines.

## Behaviours

### `default`

The scenario defines only this behaviour. For example, with
`WT_CELL_COUNT=50` and `WT_RESTART_INTERVAL=5`, it enables apps one at a time
and records an enable metric at counts 1 through 50. It restarts the conductor
and records startup metrics at 5, 10, 15, and so on through 45 enabled apps.
The final app is enabled and measured, but the conductor is not restarted at
50 enabled apps.

An app is removed from the pending queue only after it is enabled successfully.
When no pending apps remain, the behaviour stops the scenario. If the configured
duration expires first, the scenario exits with an error instead of reporting a
partial run as complete.

## Configuration

- `WT_CELL_COUNT` defaults to `10` and sets the number of disabled apps to
  install. It must be greater than `0`.
- `WT_RESTART_INTERVAL` defaults to `0` and sets how many enables occur before
  a restart. `0` disables periodic restarts.

The scenario requires exactly one agent and must manage the conductor lifecycle,
so it does not support the shared `--connection-string` option.

## Custom metrics

- `wt.custom.conductor_startup` records startup time. Tags: `agent`, `phase`.
  Fields: `value`, `cells_total`, `cells_enabled`, `enabled_pct`.
- `wt.custom.cell_enable` records app enable time. Tag: `agent`. Fields:
  `value`, `cells_total`, `cells_enabled`, `enabled_pct`.

## Suggested command

```bash
RUST_LOG=info WT_CELL_COUNT=50 WT_RESTART_INTERVAL=5 \
  cargo run -p conductor_startup -- --duration 300
```

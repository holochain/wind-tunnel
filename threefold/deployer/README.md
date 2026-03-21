# Threefold Deployer

A cli tool to deploy large numbers of VMs to Threefold.

It places VMs dynamically based on node capacity and region order, creates one network per selected node, and deploys one VM workload per node containing the assigned VM count.

This enables deployment of large numbers of nodes, beyond available capacity in a single regions.

## Behavior

- Capacity is computed per node from free CRU, MRU, and SRU.
- Per-node VM capacity is `min(cpu_limited, memory_limited, disk_limited)`.
- Regions are evaluated in the exact order passed to `--regions`.
- A preflight check fails early if estimated total capacity is below `--target-vms`.
- Deployments are concurrent per node and controlled by `--max-concurrency`.
- Node failures are tolerated while work continues; the run fails only if the target VM count is not reached.

## Usage

From repository root, using the CI shell:

```bash
nix develop .#ci --command deployer \
  --mnemonic "$THREEFOLD_TFCHAIN_WALLET_MNEMONIC" \
  --network main \
  --target-vms 120 \
  --vm-cpu 2 \
  --vm-memory-gb 8 \
  --vm-disk-gb 20 \
  --max-concurrency 8 \
  --regions "europe,americas,asia,oceania,africa" \
  --flist "https://hub.threefold.me/holochain.3bot/ghcr.io-holochain-wind-tunnel-runner-threefold-latest.flist" \
  --entrypoint "/entrypoint.sh" \
  --prefix "wt_${GITHUB_RUN_ID}"
```

From `threefold/deployer` directly:

```bash
go run . --help
```

## Configuration

- `--mnemonic` (required): Threefold wallet mnemonic.
- `--network` (default: `main`): Threefold network name.
- `--target-vms` (required): total number of VMs to deploy.
- `--vm-cpu` (default: `2`): per-VM CPU requirement.
- `--vm-memory-gb` (default: `8`): per-VM memory requirement in GiB.
- `--vm-disk-gb` (default: `20`): per-VM rootfs disk requirement in GiB.
- `--max-concurrency` (default: `6`): max concurrent node deployments.
- `--regions` (default: `europe,americas`): comma-separated placement order.
- `--flist` (required): VM flist URL.
- `--entrypoint` (default: `/entrypoint.sh`): VM entrypoint.
- `--prefix` (default: `wt_runner`): deployment/VM name prefix; alphanumeric and underscore only, max length 18.

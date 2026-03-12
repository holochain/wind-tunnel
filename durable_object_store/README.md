# Wind Tunnel Durable Object Store

A Cloudflare Worker that provides temporary key-value storage for Wind Tunnel
scenarios that need to share data between agents during a run.

Deployed at: <https://wind-tunnel-durable-objects.holochain.org>

Some scenarios require agents to share state with each other outside of the
Holochain network. This worker provides a simple Cloudflare Durable Object that
stores a single JSON blob per `run_id`. Data is automatically deleted 12 hours
after it is written.

## API

### Store a value

Storing a value in the store requires a `POST` with JSON data including the
following fields:

* `"run_id": "<string>"`: Used as a key in the value store, should be the Run
  ID of the scenario.
* `"value": <JSON blob>`: Any JSON blob, stored as the value in the store.
* `"secret": <SECRET_KEY>`: The secret required to allow storing data in the
  store which must match the vaule of the `SECRET_KEY` environment variable in
  the Cloudflare Worker. It is stored in the team's shared password vault, as
  well as in Nomad as a variable `UNYT_DURABLE_OBJECTS_SECRET`.

A valid `POST` call returns `{ "success": true }` on success.

### Retrieve a value

The Run ID of the scenario to get the data for must be provided as a query
parameter in the `GET` request, no authentication is required.
Returns `{ "value": <stored JSON> }`, or `{ "error": "<error message>" }`.

## Configuration

The `SECRET_KEY` secret must be set in Cloudflare before deploying:

```bash
wrangler secret put SECRET_KEY
```

After updating the `SECRET_KEY`, the value of `UNYT_DURABLE_OBJECTS_SECRET` in
the shared password vault must be updated along with the Nomad Variable
under the same name located at
<https://nomad-server-01.holochain.org:4646/ui/variables/var/nomad/jobs@default>.

## Local testing with scenarios

The Wind Tunnel Nix devShell provides a `local-durable-objects` command that
starts this worker locally with the same URL and secret that scenarios expect.

Run it from the repository root with:

```bash
nix run .#local-durable-objects
```

See the [unyt_chain_transaction scenario README](../scenarios/unyt_chain_transaction/README.md)
for an example of how this store is used.

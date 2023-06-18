## Schedule failure

?

## Out of capacity

?

## Server downtime

?

## Client disconnects

**Outdated**

We give clients 30s to respond to a heartbeat. This is very high in order to allow for spotty connections. See `tf/packer/static/nomad-config.hcl.tpl`.

If a client does disconnect, the Nomad server will assume the job is still running for 5 minutes before issuing a kill command. See `svc/mm-config-version-prepare/src/main.rs`. This allows for the client to disconnect from a network failure and continue to allow players to connect to running lobbies. Nodes should receive a drain command before shutting down to prevent this behavior

## Job shutdowns

We give a 60 second `kill_timeout` to jobs before sending a 137. See `svc/mm-config-version-prepare/src/main.rs`.


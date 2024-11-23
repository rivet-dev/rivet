# Troubleshooting

## Where are pegboard-manager logs?

```bash
cat /var/lib/rivet-client/logs
```

## Where are rivet-isolate-v8-runner logs?

```bash
cat /var/lib/rivet-client/runner/logs
```

## Why don't my runner logs exist?

If there are no logs at `/var/lib/rivet-client/runner/logs`, the runner binary likely failed to spawn.

Common causes:

- The path to the binary is incorrect
- Error loading libraries
- The binary is not set as executable
- The binary is for the wrong architecture

Trying to manually find and run the binary usually resolves these issues.

## `fdb ping missed`

The `rivet-client` container in `docker/dev-full/docker-compose` has the `fdbcli` CLI installed.

Check that the cluster can be connected to with:

```bash
fdbcli -C /var/lib/rivet-client/fdb.cluster --exec status
```

For further troupbleshooting, see [FoundationDB troubleshooting](../fdb/TROUBLESHOOTING.md).

## Getting logs of crashed client in Docker

If the container crashes, the logs have to be extracted from the volume.

If log redirection is enabld (you'll see the log `Redirecting all logs to /var/lib/rivet-client/log`), the logs have to be extracted from the volume since the container is down.

For example, to read the log from the volume `dev-full_client-data`, run this:

```bash
docker run --rm -it -v dev-full_client-data:/var/lib/rivet-client busybox cat /var/lib/rivet-client/log
```


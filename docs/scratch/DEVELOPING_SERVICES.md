# Developing Services

## Reading logs

### Getting started

1. Navigate to the explore section of the [Grafana dashboard](https://rivetgg.grafana.net/explore).
1. Change the data source to `rivet-loki`.
1. Flip the toggle from _Builder_ to _Code_.
1. Type: `{ns="my-namespace"}` and press enter
1. You'll see the logs from all of the services in the cluster

### Querying logs for a specific service

To query logs for a specific service such as `user-get`, write:

```logql
{ns="my-namespace",service="rivet-user-get"}
```

Note that the `service` parameter is prefixed with `rivet-` before the service name.

### Querying for all errors

To show all errors across all services, query:

```logql
{ns="my-namespace"} |= "ERROR"
```

### Querying a specific ray

To query all logs for a specific ray, query:

```logql
{ns="my-namespace"} |= "my-ray-id"
```

### Live logs

Click the _Live_ button in the top right. This will show the logs in real time.

## Checking for hard crashes

It's rare for Rivet services to hard crash assuming errors are being handled gracefully.

To check for a crash, either check on the job status on the Nomad dashboard or run:

```bash
bin/nomad/cmd status rivet-my-service
```

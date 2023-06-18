**TODO: MERGE WITH doc/TROUBLESHOOTING.md**

## Services

### Symptom

`faker-region` is hanging

### Problem

Your database was probably dropped and it can't find a region. It's polling the database, since sometimes there's a race condition between `region-probe` and `faker-region` in tests.

### Solution

Run `region-probe` again.

---

## Terraform

### Symptom

```
Error: Missing required argument

The argument "address" is required, but was not set.
```

### Problem

Terraform is trying to use a provider that you just removed. For example, if you used to use the `nomad` provider in a module and removed it, it's still trying to use that provider in order to query the previous state.

### Solution

Delete the resources related to that provider from the state.

---

## Nomad

### Symptom

Node won't drain.

### Problem

There's probably a batch job that hasn't succeeded that's running on the node.

### Solutino

Force drain the node. Make sure to double check what's failing to migrate first.

---

### Symptom

Nomad alloc with old version and same index as a newer alloc won't die.

### Problem

It's a bug in Nomad. I haven't looked for a GH issue related to this yet.

### Solution

Run `nomad system gc`.

---

### Symptom

On a Nomad server, the Nomad clients don't show up as connected to the server, but the nodes show up in Consul.

### Problem

`terraform state rm` was accidentally ran on old servers, so we were multiple servers trying to connect to the same server.

### Solution

Actually delete the old servers.

---

### Symptom

`missing eval alloc` error.

### Problem

The job probably can't be scheduled. Your disk space is probably exhausted if in dev.

You can disable the gc in `jobs_api::stop_job` to see the result in the dashboard.

---

### Symptom

Can't connect to upstream, returns "connection reset by host."

### Problem

The service is probably unhealthy. Consul may show the service as healthy, but try commenting out the health check and see how that affects it.

---

### Symptom

Alloc has error:

```
Time				  Type		   Description
failed to setup alloc: pre-run hook "consul_grpc_socket" failed: unable to create unix socket for Consul gRPC endpoint: listen unix /home/nathan/rivet/backend/.volumes/nomad/alloc/1632ad19-4515-9e69-6c0a-380c9d095a22/alloc/tmp/consul_grpc.sock: bind: invalid argument
```

### Problem

The maximum Unix socket length is 108 characters. See [here](https://github.com/hashicorp/nomad/issues/6516).

### Solution

Find a shorter path or make a symlink.

---

### Symptom

Can't access upstream.

### Problem 1

Consul can't reach the other nodes.

### Solution 1

Run `consul members` on the node to check if it's part of the cluster.

### Problem 2

The sidecar task failed to start.

### Solution 2

Check the sidecar task and see if it's running.

---

### Symptom

Nomad servers become unhealthy and memory/CPU is full.

### Problem

You'll probably see a message `Node heartbeat missed` and/or `Failed to connect to docker daemon` under _Client events_`.

If you try to SSH in to the machine, you may see an issue authenticating, even though the machine is on.

The machine is either out of memory or has a pinned CPU so it can't

### Solution

Give your services more memory, don't try to over-pack your servers.

---

### Symptom

500 internal error

### Problem

There are some cases where this is due to a bad jobspec.

### Solution

Run `nomad monitor` and apply the job again to look for errors.

Examples:

-   Attempting to convert system job to a service

---

### Symptom

Can't connect to an upstream client.

You may see that only services on the same node as the upstream client are able to connect.

### Problem

Either 8502 or some or the sidecar ports are not accessible on the other node.

### Solution

Check your firewall configuration.

---

### Symptom

Jobs stuck on starting.

### Problem

This is usually because the job depends on hardware resources that are not available.

#### Static IP

The job may have a static IP associated with it, so in order to start a rolling deploy, it has to wait for an open static port. Therefore, if your count is equal to the number of nodes you have, it'll wait indefinitely for a node with an open IP.

#### Volumes

The volume being requested may not exist.

### Solution

#### Static IP

Make sure the expected service count is at least 1 less than the total number of nodes so one deployment can become successful.

#### Volumes

Make sure that the client class being targeted has the volume you're requesting.

---

### Symptom

Querying evaluations returns 404.

### Problem

This is a bug with Nomad's federation. Evaluations will only show up in requests to servers in the same region that the job was submitted to

### Solution

Have a Nomad access server for each region you want to poll evaluations in.

---

### Symptom

No leader elected for cluster.

### Problem

Nomad depends on the Consul cluster, so this is likely a problem with Consul.

### Solution

Run `bin/cluster-recover.sh`.

If that doesn't work, see [here](https://learn.hashicorp.com/tutorials/nomad/outage-recovery).

---

### Symptom

Even on the Envoy proxy:

```
2021-08-09T05:12:18Z  Driver Failure  Failed to pull `envoyproxy/envoy:v${NOMAD_envoy_version}`: API error (400): invalid tag format
```

### Problem

[Unknown](https://github.com/hashicorp/nomad/issues/9887)

### Solution

Re-submit the job.

---

## DigitalOcean

### Symptom

cloud-init script not running correctly.

### Problem

Firewall was applied in race condition in Terraform with the droplet booting. That firewall prevented the cloud-init script from accessing a resource it needed to.

### Solution

Make sure the firewall on the droplet is valid and doesn't break any cloud-init functionality.

---

## CockroachDB

### Symptom

Migrations are hanging and the node can't be accessed in any way (including checking node status). `sqlx db setup` doesn't crash but hangs forever.

### Problem

The database is not initiated or cannot connect to peers.

### Solution

Make sure the CRDB init job ran successfully.

---

### Symptom

```
unable to GC log files: readdirent: input/output error
```

### Problem

???

### Solution

Try reverting?

---

## NATS/Chirp

### Symptom

RPC not responding.

### Problem

The worker is probably listening in a different region.

### Solution

Check that the keys that are being published on and listened on are the same.

---

## Traefik

### Symptom

502 error from services

### Problem

Either:

-   Service is down
-   The endpoint is pointed at a nonexistent location

---

## Proto

### Symptom

Proto compilation fails when defining an optional field along with a nested message/enum inside of a message.

### Problem

Not sure, seems that the prost builder fails when there is an optional field and nested message/enum defined in the same message.

### Solution

~~Move message/enum definition outside of parent message.~~
Upgrade `proto-build` to 0.8.0+.

---

# Cloudflare

### Symptom

526 Invalid SSL Cert

### Problem

Something is wrong with the Traefik configuration so it's getting a 404 and doesn't resent an mTLS cert. If you turn off `Full (Strict)` SSL encryption on Cloudflare, it'll giv eyou a 404.

### Solution

Fix the Treafik config.

---

# TypeScript/Browser/IndexedDB

### Symptom

The Storage subtab under Dev Tools > Application shows that storage keeps increasing with every refresh of a storage key.

### Problem

Chrome seemingly keeps growing the size of IndexedDB up to 4MB before resetting.
https://github.com/pouchdb/pouchdb/issues/7100#issuecomment-446848433

### Solution

No solution, this is a bug within Chrome.

# API endpoint function "higher-ranked lifetime" bug

### Symptom

```
error: higher-ranked lifetime error
 --> api-group/src/main.rs:4:2
  |
4 |     start(api_group::route::handle);
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: could not prove `impl std::future::Future<Output = std::result::Result<http::response::Response<hyper::body::body::Body>, http::error::Error>>: std::marker::Send`

error: could not compile `api-group` due to previous error
```

### Problem

This is most likely because the iterator given to `futures_util::stream::iter` is not owned.

### Solution

Change

```rust
futures_util::stream::iter(my_vec.iter())
```

to

```rust
futures_util::stream::iter(my_vec.iter().cloned())
```

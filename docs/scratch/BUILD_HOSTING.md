# Job Build Hosting

## Configuration

Job build hosting can be configured to pull directly from S3 or using a pull-through cache within the data center. We recommend using a pull-through cache for performance (and sometimes cost savings), but this requires more servers and complexity to run yourself.

## Storage

Images are stored as TAR archives, not individual layers. We do this in order to optimize download times and simplify architecture.

While it's nice to have shared base layers _in theory_, containers rarely share the exact same base layer and often causes longer download times.

Additionally, storing Docker images as layers requires running our own Docker registry. This is an extra point of failure, complexity, performant foot-guns, and cost compared to a simpler architecture with just Apache Traffic Server & S3.

## Implementation with pull-through cache

See `svc/pkg/mm/worker/src/workers/lobby_create/mod.rs` for details

### Direct from S3

When requesting a Docker build using just S3, it goes through these steps:

1. mm-lobby-create presigns an S3 request
1. Nomad makes HTTP request for the artifact
1. Nomad unpacks the TAR
1. The container starts

### Pull-through cache

Job run builds are hosted behind Apache Traffic Server within the data-center as a pull-through cache to S3. 

1. mm-lobby-create provides an internal URL to Apache Traffic Server
1. Nomad makes HTTP request for the artifact over the Nebula network
1. Nomad unpacks the TAR
1. The container starts

## Future Plans

We can't dynamically update firewalls to include the autoscaling job servers on
all of our VPS providers, so this is the best option available to securely host
Docker images.

In the future, we'll have a custom plugin for whitelisting incoming IPs in
addition to HTTP authentication in order to validate job server.

Additionally, we'll eventually choose the optimal regions to run ATS in so we're
not over-paying for block storage for ATS caching.

## Why not Consul Connect

We can't use Consul Connect to authenticate this since game nodes have extreamly
limited resources and can't run a Consul Connect instance.

# Docker Image Delivery

## Configuration

Job build hosting can be configured to pull directly from S3 or using a pull-through cache within the data center. We recommend using a pull-through cache for performance (and sometimes cost savings), but this requires more servers and complexity to run yourself.

## Storage

Images are stored as TAR archives, not individual layers. We do this in order to optimize download times and simplify architecture.

While it's nice to have shared base layers _in theory_, containers rarely share the exact same base layer and often causes longer download times.

Additionally, storing Docker images as layers requires running our own Docker registry. This is an extra point of failure, complexity, performant foot-guns, and cost compared to a simpler architecture with just Apache Traffic Server & S3.

## Implementation

See [`svc/pkg/mm/worker/src/workers/lobby_create/mod.rs`](/svc/pkg/mm/worker/src/workers/lobby_create/mod.rs) for details

### Direct from S3

When requesting a Docker build using just S3, it goes through these steps:

1. mm-lobby-create presigns an S3 request
1. Nomad makes HTTP request for the artifact
1. Nomad unpacks the TAR
1. The container starts

### Pull-through cache

Job run builds are hosted behind Apache Traffic Server within the data-center as a pull-through cache to S3.

1. mm-lobby-create provides an internal URL to Apache Traffic Server
1. Nomad makes HTTP request for the artifact over VLAN
1. Nomad unpacks the TAR
1. The container starts

## Why don't we gzip Docker images?

Two things are incredibly important:

-   Lobby startup performance
-   Disk space on nodes

Storing images as gzipped files requires them to be extracted when loaded. This means that you need to wait for the file to be extracted (which may be a long time for large images) and will take double the disk space to load the image.

We use gzip in transit, so it doesn't make a significant difference.

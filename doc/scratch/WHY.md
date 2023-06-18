# Why...

## ...do we not gzip Docker images?

Two things are incredibly important:

-   Lobby startup performance
-   Disk space on nodes

Storing images as gzipped files requires them to be extracted when loaded. This means that you need to wait for the file to be extracted (which may be a long time for large images) and will take double the disk space to load the image.

We use gzip in transit, so it doesn't make a significant difference.

## ...don't we use CSI?

CSI adds many many more points of failure. Nomad especially has a lot of bugs related to CSI

-   [CSI mounting issue](https://github.com/hashicorp/nomad/issues/10927#issuecomment-905859501).
-   Even if the volume was mounted, it often would be able to be read or written to

## ...don't we use Prometheus remote write?

Web push is fine, but it's just another potential point of failure for monitoring. Directly connecting to the Prometheus agent from the master node has less potential for failure.

## ...are log files so small?

Promtail likes to eat memory and is best friends with the OOM reaper. Smaller log files make it easier for Promtail to track positions and upload under low memory constraints

## ...do we disable caches (dual logging) for Docker logging?

Docker dual logging caches logs in memory no matter the logging driver in order to ensure that `docker logs` works no matter what. The command will cache [up to 100 MB](https://docs.docker.com/config/containers/logging/dual-logging/#configuration-options) by default. We never use that command and we can't afford that overhead when running this many services, so we disable it.

Additionally, it seems that after disabling the command, our dockerd memory usage in development no longer creeps up when running many job containers.

## ...don't we use Docker logging drivers?

These have caused countless issues from dockerd memory leaks, services failing to boot (both with syslog and Loki plugins), etc.

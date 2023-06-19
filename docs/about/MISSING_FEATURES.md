# Notable Missing Features

Have a feature request you don't see here? File an issue!

## Standalone production setup guide

We don't have a formal guide on how to deploy a production instance of Rivet yourself.

## Multiple clouds

The open source version of Rivet only supports Linode at the moment. As we progress, we'll be bringing our other cloud providers to open source soon.

## BYO job servers

Rivet does not support bringing your own job servers (regardless of cloud) at the moment. This will eventually let you run your own hardware, including that toaster sitting in your basement.

## Slow development setup times

It takes a long time to bootstrap a standalone Rivet cluster for development. Like _really_ long. Trust us, we get it.

## ClickHouse failover & sharding

Self-hosted ClickHouse doesn't have a failover or sharding mechanism at the moment.

## Traffic Server `consistent_hash` routing

SVC-824

## Sharable Grafana dashboards

Rivet exposes metrics and logs for everything, but we don't publish Grafana internal dashboards.

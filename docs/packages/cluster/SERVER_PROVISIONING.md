# Automatic Server Provisioning

Server provisioning handles everything responsible for getting servers running and installed for game lobbies to run on. Server provisioning occurs in the `cluster` package and is automatically brought up and down to desired levels via `cluster-datacenter-scale`.

## Motivation

Server provisioning was created to allow for quick and stateful configuration of the game server topology on Rivet. This system was also written with the intention to allow clients to choose their own hardware options and server providers.

In the future, an autoscaling system will be hooked up to the provisioning system to allow the system to scale up to meet spikes in demand, and scale down when load is decreased to save on costs.

## Basic structure

There are currently three types of servers that work together to host game lobbies:

-   ### ATS

    ATS servers host game images via Apache Traffic server. The caching feature provided by ATS along with ATS node being in the same datacenter as the Job node allows for very quick lobby start times.

-   ### Job

    Job servers run Nomad which handles the orchestration of the game lobbies themselves.

-   ### GG

    GameGuard nodes serve as a proxy for all incoming game connection and provide DoS protection.

## Why are servers in the same availability zone (aka datacenter or region)

Servers are placed in the same region for two reasons:

1. ### VLAN + Network Constraints

    Servers rely on VLAN to communicate between each other.

2. ### Latency

    Having all of the required components to run a Job server on the edge, (i.e. in the same datacenter) allows for very quick lobby start times.

## Prior art

-   https://console.aiven.io/project/rivet-3143/new-service?serviceType=pg
-   https://karpenter.sh/docs/concepts/nodepools/
-   Nomad autoscaler

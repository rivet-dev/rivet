# Automatic Server Provisioning

Server provisioning handles everything responsible for getting servers from supported cloud providers running and installed with all of the software required to run Rivet edge functionality. Server provisioning occurs in the `cluster` package and server count is automatically brought up and down to desired levels via `cluster-datacenter-scale`.

Server provisioning is declarative, meaning it is configured based on the state you want the [cluster](#cluster) (all [datacenters](#datacenter) and servers within datacenters) to be in. The operations required to get the current cluster state to match the desired state are handled automatically.

## Motivation

Server provisioning was created to allow for quick and stateful configuration of the game server topology on Rivet. This system was also written with the intention to allow clients to choose their own hardware options and server providers.

On Rivet Enterprise, an autoscaling system is hooked up to the provisioning system to allow the system to scale up to meet spikes in demand, and scale down when load is decreased to save on costs.

## Basic structure

There are currently three types of servers, known as [pools](#pool), that work together to host game lobbies:

-   ### ATS

    ATS servers host game images via Apache Traffic Server. The caching feature provided by ATS along with ATS node being in the same datacenter as the [Job](#job) node allows for very quick lobby start times.

-   ### Job

    Job servers run Nomad which handles the orchestration of the game lobbies themselves.

-   ### GG

    Game Guard nodes serve as a proxy for all incoming game connection and provide DDoS protection.

## Provisioning process (upscaling)

If `cluster-datacenter-scale` determines that there are less servers in a [pool](#pool) than the desired count, it will provision new servers or [undrain](#drainundrain) currently draining servers.

-   ### Creating a new server

    1. Before the new server is provisioned, it checks if a [prebaked](#prebaking) image for the given [pool](#pool) already exists. If it does, the prebake image is copied to the newly created disk and no install procedure is required. If the prebake image does not exist, it will be created on a separate prebake server. The current server being created will be ssh'd into and run install scripts that are customized based on the [pool](#pool) this server is assigned to.

-   ### [Prebaking](#prebaking)

    The process for prebaking a server image is the same as installing but without initialization. A new server is created and installed with the software required by the [pool](#pool) type, but none of the software is turned on. The server is then shut down, and an image of the disk is created. Finally, the image id is written to database and the server is deleted.

-   ### [Undraining](#drainundrain)

    A server that is currently draining (usually from [downscaling](#drainingdestroying-process-downscaling)) can be undrained to get it back to its normal state. This is preferred over creating a new server because it is much faster.

## Draining/destroying process (downscaling)

If `cluster-datacenter-scale` determines that there are more servers in a [pool](#pool) than the desired count, it will delete or [drain](#drainundrain) servers.

-   ### Deleting servers

    Servers are deleted by destroying all related resources such as DNS records, SSH keys, and firewalls before finally deleting the server itself via the cloud provider's API.

-   ### [Draining](#drainundrain)

    A server is drained to allow it to finish pending operations or allow game lobbies to close gracefully before it is destroyed. In this state, it can be undrained.

## Tainting

A [datacenter](#datacenter) can be tainted to allow for a rolling deploy of new changes to the underlying software configuration within install scripts. When tainted, all of the servers in the datacenter will be marked as tainted and the same amount of new servers will be deployed. These tainted servers do not differ in functionality from normal servers. However, once the new servers begin to come online, the tainted servers start to get [drained](#draining) until all tainted servers are drained/deleted.

## Why are servers in the same [availability zone](#availability-zone)

Servers are placed in the same [AZ](#availability-zone) for two reasons:

1. ### VLAN + Network Constraints

    Servers rely on VLAN to communicate between each other.

2. ### Latency

    Having all of the required components to run a [Job](#job) server on the edge, (i.e. in the same [datacenter](#datacenter)) allows for very quick lobby start times.

## Prior art

-   https://console.aiven.io/project/rivet-3143/new-service?serviceType=pg
-   https://karpenter.sh/docs/concepts/nodepools/
-   Nomad autoscaler

## Terminology

-   #### Cluster

    A collection of datacenters.

-   #### Datacenter

    A collection of servers in the same availability zone of a cloud server provider.

-   #### Pool

    A pool is a collection of servers with the same purpose. Read more [here](#basic-structure).

-   #### Availability zone

    Also known as region or datacenter.

-   #### Drain/Undrain

    When a server is drained, it is put in a state in which it can complete all remaining operations before being deleted. When a draining server is undrained, it is set back to a state of normal function.

-   #### Prebaking

    Prebaking refers to the process of installing a variation of the required software for the given [pool](#pool) on a prebake server to create a prebake image. It must be a variation because the prebake image cannot know what server it will be copied to. Can be though of a template.

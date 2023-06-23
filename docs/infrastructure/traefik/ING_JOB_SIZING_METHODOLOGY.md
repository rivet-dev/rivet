# ing-job Sizing Methodology

ing-jobs favor fewer, more powerful servers (i.e. faster) as opposed to more, less powerful servers (i.e. higher fault tolerance).

## Vulnerabilities

### OOM crashes

Sufficient DDoS attacks can exhausts Traefik's memory and cause an OOM crash. This disconnects all clients, which is really bad.

Since attackers focus their resources on one node at a time, we need to make it as hard as possible to take down.

### Network saturation

DDoS attacks can flood Traefik with large amounts of packets and degrades the experience for other clients.

Since this doesn't actually crash servers, this approach favors many smaller nodes where an attacker would need to saturate the network on all nodes to cause a service outage. With larger nodes, it's easier to degrade performance on a single node.

## Why use larger nodes

### Benefits

-   Attackers focus their resources on one node at a time
-   Larger nodes take more resources to take down
-   When distributing connections at random, there's more buffer to account for an imbalance as compared to smaller nodes

### Detriments

-   This is at the expense of having a larger impact if the node is successfully crashed or has to be migrated
-   It's easier to saturate the network for a single load balancer

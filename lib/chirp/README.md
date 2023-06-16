# Chirp
## Load balancing
NATS will send messages randomly to members of a queue. This is a good enough load balancer, since our current workers have fairly lightweight workloads.

We rely on Nomad's CPU autoscaler to automatically scale to meet the demand.


# Consul

## What is Consul?

Consul is a central registry of any service that's running. This is commonly referred to as "service discovery" software. Consul also provides a DNS server that we use to easily reach our services and health checks that automatically manage routing to our services.

For example, our Cockroach database is reachable at `sql.cockroach.service.consul` within our cluster.

## What do we use Consul for?

We use Consul to monitor the health of our services, use DNS to reach our services, manage service tags that Traefik reads, and provide Prometheus with all the services to monitor.

## Dashboard

You can reach the Consul dashboard at `https://consul.my-namespace.gameinc.io`.


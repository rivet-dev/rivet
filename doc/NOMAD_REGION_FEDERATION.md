# Nomad Region Federation

Multi-region deployments is an [enterprise-only feature](https://learn.hashicorp.com/tutorials/nomad/federation).

Therefore, we create a new service that gets deployed to each region manually. For example, the service `crdb` becomes `crdb:do-sfo` and `crdb:do-fra`.

The main downside to this is that regions can't coordinate blue-green deployments and we can't use auto-reverting services, since we'll end up with two clusters with different versions of a service.


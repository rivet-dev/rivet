# Internal <-> External Naming Aliases

Most internal types have different names than what's exposed in the public API.

This allos us to start devleoping functionality internally and decide the best name for DX after the fact.

## Aliases

| Internal                                 | External       | Notes                                                                                                                                                                                                                                              |
| ---------------------------------------- | -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ds` (Dynamic Server) & Pegboard `actor` | Actors         | `ds` glues together `cluster` and `pegboard`                                                                                                                                                                                                       |
| `pegboard`                               | Orchestrator   |                                                                                                                                                                                                                                                    |
| `pegboard-manager`                       | Rivet Client   |                                                                                                                                                                                                                                                    |
| Region                                   | Datacenter     | Regions will eventually be split from datacenters internally for failover support. This will not be a breaking change since we refer to regions by the datacenter name ID. We currently have an internal `region` package, but this is deprecated. |
| `team`                                   | Developer Team |                                                                                                                                                                                                                                                    |
| `identity`                               | User           |                                                                                                                                                                                                                                                    |
| `game`                                   | Project        |                                                                                                                                                                                                                                                    |
| `namespace`                              | Environment    |                                                                                                                                                                                                                                                    |

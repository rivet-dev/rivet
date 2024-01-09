# job-runner

This crate is used to run OCI bundles on the job servers themselves. This takes
care of trapping signals, log shipping, rate limiting logs, and more.

## Deployment

This gets built & deployed in `infra/tf/infra-artifacts/` then used in `svc/pkg/mm/worker/src/workers/lobby_create/nomad_job.rs`.

# Job draining & kill timeouts

## Relavant Code

| Name                           | Timeout                            | Reason                                                                                              | Location                                                                                                          |
| ------------------------------ | ---------------------------------- | --------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Nomad client config            | Something really high              | Always be higher than anything passed to `datacenter.drain_timeout`                                 | `svc/pkg/cluster/worker/src/workers/server_install/install_scripts/files/nomad_configure.sh` (`max_kill_timeout`) |
| Drain nomad job                | `datacenter.drain_timeout`         | How long the Nomad jobs have to stop                                                                | `svc/pkg/mm/worker/src/workers/lobby_create/nomad_job.rs` (`nodes_api::update_node_drain`)                        |
| Nomad job kill timeout         | Something really high              | Always be higher than anything passed to `datacenter.drain_timeout`. We'll manually send `SIGKILL`. | `svc/pkg/mm/worker/src/workers/lobby_create/nomad_job.rs` (`kill_timeout`)                                        |
| job-run-stop delete Nomad job  | Nomad job kill timeout (see above) | This causes Nomad to send a `SIGTERM`                                                               | `svc/pkg/mm/worker/src/workers/lobby_create/nomad_job.rs` (`kill_timeout`)                                        |
| job-run-stop manually kill job | `util_job::JOB_STOP_TIMEOUT` (30s) | This lets us configure a lower kill timeout when manually stopping a job                            | `svc/pkg/job-run/worker/src/workers/stop.rs` (`allocations_api::signal_allocation`)                               |

## Signals 101

- `SIGTERM` = gracefully stop, jobs should handle this gracefully
- `SIGKILL` = hard stop, cannot be handled custom

## Node draining vs manually stopping a job

### Node draining

1. `nodes_api::update_node_drain`
2. Calls `SIGTERM` on jobs PROBLEM: jobs are only given 60s to shut down b/c of their `kill_timeout`
3. Waits until the timeout
4. Sends `SIGKILL` to any remaining jobs

### Manually stopping a job

1. `allocations_api::delete_job`, which Nomad sends `SIGTERM`
2. Manually send `SIGKILL` after `util_job::JOB_STOP_TIMEOUT` if alloc still running
   - This is less than the job's kill timeout
   - If the worker crashes, job-gc will clean up the job later

# Nomad

## What is Nomad?

Nomad is a piece of software that lets us run jobs (e.g. Docker containers, CRON jobs, virtual machines, etc)
across multiple servers. This is commonly referred to as an orchestrator. It's similar to Kubernetes, but lets
us run more than just Docker containers and has a much simpler and more flexible architecture.

## What is Nomad used for?

We use Nomad to run our Rivet services (see [`svc/`](../../../svc)), any 3rd party services that don't require
a persistent volume or IP (see [`infra/tf/nomad/`](../../../infra/tf)), and all of the Rivet Serverless
Lobbies for our customers (see [`job-run-create`](../../../svc/pkg/job-run/worker/src/workers/create/mod.rs)).

## Why bin pack and not spread?

In the HashCorp Nomad C2M challenge, they [switched to the spread scheduler](https://www.hashicorp.com/c2m) to
increase performance.

**Autoscaling**

Rivet needs to be able to shut down nodes as game servers come and go. Rivet will not shut down running games,
so we cannot scale down a node with a game server already on it. Therefore, we need bin packing to ensure that
any server that can be made free will be made free, while spread schedulers will optimize to have a job on
each server.

**Jobs that use 100% machine resources**

Rivet does not use the spread scheduler because there are some jobs that require all of the CPUs and will not
be able to schedule if there is a smaller job running on each node with spread.

Example: we have two machines with 4 CPU cores each. We first schedule 2 jobs using 2 CPU cores each. Then we
schedule a single job with 4 CPU cores.

With bin packing, the first machine would run the jobs with 2 CPU cores each, then the second machine would
run the 4 CPU core job.

With spread, each machine would run a 2 CPU core job. Scheduling the 4 CPU core job would fail, since there is
no machine with 4 CPU cores available.

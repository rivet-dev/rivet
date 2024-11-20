# GitHub Actions

## When to use self hosted vs managed runners

**Managed runners**

Managed runners are good for running bursts of jobs in parallel, quickly.

Use these for low-CPU intensive jobs, like formatting, linting, etc. Using
these for resource-hungry jobs will make those jobs take a long time and cost a
lot of money.

**Self-hosted Runners**

Self-hosted runners are good for running hardware intensive jobs quickly.
However, they have a fixed number of nodes that can run one job at a time each,
so we want to (a) always target 100% resource usage and (b) maintain as few of
these as possible in order to keep them cheap.

Use these for CPU- & memory-intensive jobs. Using these for low-CPU intensive
jobs will cause jobs to queue up and take a long time to complete.


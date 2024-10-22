# v8-isolate-runner

This crate is used to run JavaScript on the pegboard servers themselves. This takes care of log shipping, rate
limiting logs, and more.

In contrast to the container runner which runs a single container per process, this runs multiple isolates in
a single v8 runtime.

## Deployment

This gets built & deployed in `infra/tf/infra-artifacts/` then used in `TODO`.

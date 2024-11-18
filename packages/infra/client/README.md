# Rivet Client

## Projects

- **manager** The binary responsible for talking to the Rivet Server. This will spawn a runner based on the flavor (isolate or container).
- **isolate-v8-runner** Runs actors using V8 isolates.
- **container-runner** Runs actors as containers.
- **runner-protocol** Shared types for the runner's protocol.
- **echo** Used as a test binary for testing pegboard-manager.

## rustls and OpenSSL

We opt to use rustls instead of OpenSSL in all client binaries in the interest of portability.


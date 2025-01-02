# Rivet Client

## Projects

- **manager** The binary responsible for talking to the Rivet Server. This will spawn a runner based on the flavor (isolate or container).
- **isolate-v8-runner** Runs actors using V8 isolates.
- **container-runner** Runs actors as containers.
- **actor-kv** API layer between FoundationDB and actors.
- **logs** Simple file-descriptor-based logger with rotation and retention.
- **config** Config definitions for all client components.
- **echo** Used as a test binary for testing pegboard-manager.

## rustls and OpenSSL

We opt to use rustls instead of OpenSSL in all client binaries in the interest of portability.


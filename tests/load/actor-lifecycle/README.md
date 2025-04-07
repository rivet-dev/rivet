# Basic

Runs a basic load test that:

- Creates an actor
- Connects to actor over WS
- Sends ping & waits for pong
- Destroys actor

## Usage

This is intended to be used with `system-test` (in `examples/system-test/`).

The build name is specified with `BUILD`.

**Test Isolates**

```
BUILD=ws-isolate just k6-run actor-lifecycle
```

**Test Container**

```
BUILD=ws-container just k6-run actor-lifecycle
```

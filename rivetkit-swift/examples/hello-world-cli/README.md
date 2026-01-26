# Hello World (CLI)

A minimal CLI Swift example that connects to the same counter actor used by the SwiftUI example. It demonstrates async actions and event subscriptions without any UI.

## Server

```bash
cd rivetkit-swift/examples/hello-world-cli/server
pnpm install
pnpm dev
```

The server listens on `http://127.0.0.1:8787` and exposes RivetKit at `http://127.0.0.1:8787/api/rivet`.

## CLI Client

```bash
cd rivetkit-swift/examples/hello-world-cli/client
swift run
```

Override the endpoint if needed:

```bash
RIVET_ENDPOINT=http://<host-ip>:8787/api/rivet swift run
```

Set a custom actor key:

```bash
RIVET_ACTOR_KEY=my-key swift run
# or
swift run -- --key my-key
```

## Notes

- The client subscribes to `newCount` and prints each update.
- It increments the counter a few times before exiting.

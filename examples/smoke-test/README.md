# Smoke Test for RivetKit

Example project demonstrating a simple getOrCreate smoke test with [RivetKit](https://rivetkit.org).

[Learn More →](https://github.com/rivet-dev/rivetkit)

[Discord](https://rivet.dev/discord) — [Documentation](https://rivetkit.org) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Getting Started

### Prerequisites

- Node.js

### Installation

```sh
git clone https://github.com/rivet-dev/rivetkit
cd rivetkit/examples/smoke-test
npm install
```

### Development

```sh
npm run dev
```

Run the smoke test to exercise multiple actor creations:

```sh
npm run smoke
```

Set `TOTAL_ACTOR_COUNT` and `SPAWN_ACTOR_INTERVAL` environment variables to adjust the workload.

## License

Apache 2.0

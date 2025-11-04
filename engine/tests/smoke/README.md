# Smoke Test for RivetKit

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

Set `BEHAVIOR` to change the test type.


# Deno Example for RivetKit

Example project demonstrating basic actor state management and RPC calls with [RivetKit](https://rivetkit.org) using Deno runtime.

[Learn More →](https://github.com/rivet-dev/rivetkit)

[Discord](https://rivet.dev/discord) — [Documentation](https://rivetkit.org) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Getting Started

### Prerequisites

- Deno

### Installation

```sh
git clone https://github.com/rivet-dev/rivetkit
cd rivetkit/examples/deno
pnpm install
```

**Notice:** We use `pnpm install` here as Deno offers compatability with package.json via npm/pnpm. Some packages used in rivetkit are simpler to install with npm/pnpm.

### Development

```sh
deno task dev
```

Run the connect script to interact with the counter:

```sh
deno task connect
```

## License

Apache 2.0
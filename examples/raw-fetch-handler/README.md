# Raw Fetch Handler Example for Rivet

Example project demonstrating raw HTTP fetch handling with Hono integration in [Rivet](https://www.rivet.dev/).

[Learn More →](https://github.com/rivet-dev/rivet)

[Discord](https://rivet.dev/discord) — [Documentation](https://www.rivet.dev/) — [Issues](https://github.com/rivet-dev/rivet/issues)

## Overview

This example demonstrates:
- Using Hono router inside an actor's `onFetch` handler via `createVars`
- Creating named counter actors that maintain independent state
- Making fetch requests to actors through the frontend client
- Forwarding requests from custom Hono endpoints to actor fetch handlers
- Building a React frontend that interacts with Rivet actors
- Testing actors with fetch handlers

## Project Structure

```
raw-fetch-handler/
├── src/
│   ├── backend/     # Rivet server with counter actors
│   └── frontend/    # React app demonstrating client interactions
└── tests/           # Vitest test suite
```

## Getting Started

### Prerequisites

- Node.js

### Installation

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/raw-fetch-handler
pnpm install
```

### Development

Start both backend and frontend:

```sh
pnpm dev
```

Or run them separately:

```sh
# Terminal 1 - Backend
pnpm dev:backend

# Terminal 2 - Frontend
pnpm dev:frontend
```

Run tests:

```sh
pnpm test
```

## Features

### Backend

1. **Counter Actor** - A simple counter with HTTP endpoints
   - `GET /count` - Get current count
   - `POST /increment` - Increment the counter

2. **Forward Endpoint** - Routes requests to actor fetch handlers
   - `/forward/:name/*` - Forward any request to the named actor

### Frontend

A React app demonstrating:
- Creating multiple named counters
- Interacting via actor fetch API
- Using the forward endpoint
- Real-time state updates

## How It Works

1. The backend defines a counter actor with a Hono router
2. Each counter is identified by a unique name
3. The frontend can interact with counters in two ways:
   - Direct actor fetch calls using the Rivet client
   - HTTP requests through the forward endpoint
4. Multiple counters maintain independent state

## License

Apache 2.0
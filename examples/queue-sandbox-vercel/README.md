> **Note:** This is the Vercel-optimized version of the [queue-sandbox](../queue-sandbox) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fqueue-sandbox-vercel&project-name=queue-sandbox-vercel)

# Queue Sandbox

Interactive demo showcasing all the ways to use queues in RivetKit. Each tab demonstrates a different queue pattern with real-time feedback.

## Getting Started

```bash
cd examples/queue-sandbox
pnpm install
pnpm dev
```

## Features

- Six interactive tabs demonstrating different queue patterns
- Real-time state updates via broadcasts and polling
- Progress indicators for long-running operations
- Multi-queue priority handling

## Implementation

This example demonstrates six queue patterns:

### Send

Basic queue messaging where the client sends messages to an actor queue, and the actor manually receives them.

See [`src/actors/sender.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/queue-sandbox/src/actors/sender.ts).

### Multi-Queue

Listen to multiple named queues (high, normal, low priority) simultaneously using `c.queue.next(names, { count })`.

See [`src/actors/multi-queue.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/queue-sandbox/src/actors/multi-queue.ts).

### Timeout

Demonstrate the timeout option when waiting for messages. Shows countdown timer and handles both successful receives and timeouts.

See [`src/actors/timeout.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/queue-sandbox/src/actors/timeout.ts).

### Worker

Use the `run` handler to continuously consume queue messages in a loop. The worker polls for jobs and processes them automatically.

See [`src/actors/worker.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/queue-sandbox/src/actors/worker.ts).

### Self-Send

Actor sends messages to its own queue using the inline client pattern (`c.client<typeof registry>()`).

See [`src/actors/self-sender.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/queue-sandbox/src/actors/self-sender.ts).

### Keep Awake

Consume queue messages and perform long-running tasks wrapped in `c.keepAwake()` to prevent the actor from sleeping during processing.

See [`src/actors/keep-awake.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/queue-sandbox/src/actors/keep-awake.ts).

## Resources

Read more about [queues](/docs/actors/queues), [run handlers](/docs/actors/run), and [state](/docs/actors/state).

## License

MIT

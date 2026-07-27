# example-agent-actor

The same agent as [`example-agent-simple`](../example-agent-simple), moved into a [Rivet Actor](https://rivet.dev). The agent loop still runs outside the sandbox, but now it lives in a long-running, stateful backend process: tasks arrive on the actor's durable queue and are processed one at a time by its `run` handler, the sandbox is attached in `createVars` and its id is persisted in actor state so the actor reattaches to the same sandbox across sleeps and restarts, and the session history is stored in the actor's SQLite database.

## Prerequisites

- Node.js 20+
- Docker running locally
- `ANTHROPIC_API_KEY` set in your environment

## Run

Start the server (this also starts a local Rivet engine):

```bash
npm install
npm run dev
```

Then in another terminal, start the client:

```bash
npm run client
```

Type a task at the `agent>` prompt and the agent completes it in the sandbox. Tool calls print in the server terminal. Ctrl+C exits. Because history lives in the actor's SQLite database, you can restart the client (or the whole server) and the session picks up where it left off.

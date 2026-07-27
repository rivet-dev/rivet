# example-agent-simple

A minimal agent that runs in a plain Node.js script and talks to a sandbox through tools. The agent loop is the [Vercel AI SDK](https://ai-sdk.dev)'s `generateText` tool loop running in your process. The sandbox is a [ComputeSDK](https://computesdk.com) sandbox running on the local Docker provider.

## Prerequisites

- Node.js 20+
- Docker running locally
- `ANTHROPIC_API_KEY` set in your environment

## Run

```bash
npm install
npm start
```

Type a task at the `agent>` prompt (for example, "write a Node.js script that prints the first 10 Fibonacci numbers and run it") and the agent completes it in the sandbox, printing the tool calls it makes along the way. Ctrl+C exits. Swap the `docker` provider for any other ComputeSDK provider (E2B, Daytona, Vercel, etc.) to run against a cloud sandbox without changing the agent.

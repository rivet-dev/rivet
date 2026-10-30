# Claude Code Research

A multi-agent research team where every agent is a Claude Code run under your own Claude Code login. A coordinator plans sub-questions, researcher agents investigate them in parallel with web search, and the coordinator writes a sourced report.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/claude-code-research
pnpm install
pnpm start
```

In a second terminal:

```sh
pnpm client "How do Rivet Actors compare to Cloudflare Durable Objects for building AI agents?"
```

## Features

- **Agents as actors**: each researcher is a `claudeCodeAgent` actor that keeps its Claude Code session and turns in durable state
- **Parallel fan-out**: the coordinator calls one researcher actor per sub-question concurrently through the actor client
- **Background work**: `c.keepAwake` keeps the coordinator running while the research continues after `start` returns
- **Live progress**: the coordinator broadcasts progress events to connected clients
- **Structured output**: the planner returns validated sub-questions through a Zod schema

## Prerequisites

- Claude Code installed and logged in. Run `claude` once.

Agent runs count toward your Claude Code usage limits. Use your own login for your own local work only.

## Implementation

The researcher agent and the coordinator live in [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/claude-code-research/src/actors.ts). Researchers come from [`@rivet-dev/claude-code`](https://github.com/rivet-dev/rivet/tree/main/integrations/claude-code), and the coordinator uses `runClaudeCode` for planning and the final report.

The command line client in [`scripts/client.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/claude-code-research/scripts/client.ts) starts a research run and prints its progress.

## Resources

Read more about [actions](/docs/actors/actions), [events](/docs/actors/events), [state](/docs/actors/state), and [communicating between actors](/docs/actors/communicating-between-actors).

## License

MIT

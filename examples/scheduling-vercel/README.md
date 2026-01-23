> **Note:** This is the Vercel-optimized version of the [scheduling](../scheduling) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fscheduling-vercel&project-name=scheduling-vercel)

# Scheduling

Demonstrates how to schedule tasks and execute code at specific times or intervals using Rivet Actors.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/scheduling
npm install
npm run dev
```


## Features

- **Task scheduling**: Schedule actor actions to run at specific times with `schedule.at()`
- **Delayed execution**: Schedule tasks to run after a delay with `schedule.after()`
- **Persistent schedules**: Scheduled tasks survive actor restarts
- **Action callbacks**: Scheduled tasks invoke actor actions with custom payloads

## Implementation

This example demonstrates time-based task scheduling in Rivet Actors:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/scheduling/src/backend/registry.ts)): Shows how to use `schedule.at()` and `schedule.after()` to schedule future actions with persistent state

## Resources

Read more about [scheduling](/docs/actors/scheduling), [actions](/docs/actors/actions), and [state](/docs/actors/state).

## License

MIT

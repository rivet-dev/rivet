# @rivet-dev/flue

A [Flue](https://github.com/withastro/flue) build target that compiles a
Flue project (agents + workflows) into a [Rivet](https://rivet.dev) Actor
application. Set `target: rivet` in `flue.config.ts` and `flue build` emits a
server entrypoint that maps each agent and each workflow to a Rivet Actor, with
durable sessions, submission recovery, and inter-actor `dispatch()` backed by
each actor's native SQLite database (`c.db`).

Full documentation, setup, and configuration:
**https://rivet.dev/docs/integrations/flue**

Runnable examples: [`examples/rivet`](./examples/rivet) (a plain `target: rivet`
app with one agent and one dispatch workflow) and
[`examples/rivet-vercel`](./examples/rivet-vercel) (the same app fronted by
Next.js route handlers).

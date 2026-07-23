# @rivet-dev/flue

The Rivet Actor runtime implementation used by `@rivet-dev/flue-target`. It
provides durable sessions, submission recovery, inter-actor `dispatch()`, and
Flue storage backed by each actor's native SQLite database (`c.db`).

Application code should install and configure `@rivet-dev/flue-target` instead
of importing this package directly.

Full documentation, setup, and configuration:
**https://rivet.dev/docs/integrations/flue**

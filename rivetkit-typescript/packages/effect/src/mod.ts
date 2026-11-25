/** The `SqlClient` of `effect/sql`. An actor that declares `db` gets one for its SQLite database. */
export { SqlClient } from "effect/sql";
/** The `db` of `rivetkit/db`. Pass `db()` as the `db` option of `toLayer` to give the actor a SQLite database. */
export { db } from "rivetkit/db";
export * as Action from "./Action.ts";
export * as Actor from "./Actor.ts";
export * as Client from "./Client.ts";
export * as Event from "./Event.ts";
export * as Registry from "./Registry.ts";
export * as RivetError from "./RivetError.ts";
export * as RivetLogger from "./RivetLogger.ts";
/** @deprecated Actor state was deprecated in 3.0 and will be removed in a future version. Store durable data in SQLite (`c.db`) and in-memory data in `c.vars`. See https://rivet.dev/actors/docs/deprecated/state */
export * as State from "./State.ts";
export * as Workflow from "./Workflow.ts";

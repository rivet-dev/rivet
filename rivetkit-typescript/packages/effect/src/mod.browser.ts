/**
 * The entry that the `browser` export condition selects. It has the exports
 * of `mod.ts`, so a contract and its client code bundle without server
 * packages. The exports that need a server throw when they are used.
 */

import type * as RivetkitDb from "rivetkit/db";
import { make as makeWorkflowContract } from "./internal/WorkflowContract.ts";
import type * as RegistryServer from "./Registry.ts";
import type * as WorkflowServer from "./Workflow.ts";

const serverOnlyError = (name: string) =>
	new Error(
		`@rivetkit/effect: ${name} runs only on a server. This is the browser build of the package.`,
	);

const serverOnlyNamespace = <Namespace extends object>(
	name: string,
	browserSafe: Partial<Namespace> = {},
) =>
	new Proxy(browserSafe as Namespace, {
		get: (target, key, receiver) => {
			if (Reflect.has(target, key)) {
				return Reflect.get(target, key, receiver);
			}
			/** Tools that inspect a value read symbol keys. */
			if (typeof key === "symbol") return undefined;
			throw serverOnlyError(`${name}.${key}`);
		},
	});

export { SqlClient } from "effect/sql";
export * as Action from "./Action.ts";
export * as Actor from "./Actor.ts";
export * as Client from "./Client.ts";
export * as Event from "./Event.ts";
export * as RivetError from "./RivetError.ts";
export * as RivetLogger from "./RivetLogger.ts";
/** @deprecated Actor state was deprecated in 3.0 and will be removed in a future version. Store durable data in SQLite (`c.db`) and in-memory data in `c.vars`. See https://rivet.dev/actors/docs/deprecated/state */
export * as State from "./State.ts";

export const db: typeof RivetkitDb.db = () => {
	throw serverOnlyError("db");
};
export const Registry = serverOnlyNamespace<typeof RegistryServer>("Registry");
export const Workflow = serverOnlyNamespace<typeof WorkflowServer>("Workflow", {
	make: makeWorkflowContract,
});

export { createNoopTraces } from "./noop.js";

// Browser stub: createTraces is server-only (uses node:async_hooks, node:crypto,
// fdb-tuple). This module is selected via the "browser" export condition so that
// bundlers like Vite never pull in the real implementation when resolving
// @rivetkit/traces in a browser context.  The function is never actually called
// in the browser; it only exists because tsup chunk-splitting may place the
// import in a shared chunk also reached by client code.
export function createTraces(): never {
	throw new Error(
		"createTraces is not available in the browser. This is a server-only API.",
	);
}

/**
 * `@rivetkit/world-workflow` — Vercel Workflow SDK World implementation
 * backed by Rivet Actors.
 *
 * Usage:
 *
 * ```ts
 * import { createRivetWorld } from "@rivetkit/world-workflow";
 * import { registry } from "@rivetkit/world-workflow/registry";
 *
 * // Somewhere in your server entry point, start the registry:
 * registry.start();
 *
 * // And build a World the Workflow SDK can use:
 * export const world = createRivetWorld({
 *   endpoint: process.env.RIVET_ENDPOINT,
 * });
 * ```
 */

export { createRivetWorld } from "./world";
export type { RivetWorldConfig } from "./world";
export * from "./types";

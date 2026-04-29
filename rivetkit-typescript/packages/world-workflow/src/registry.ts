/**
 * RivetKit registry containing the three actors that back the Rivet World.
 *
 * Host code that wants to run a Rivet World must boot this registry via the
 * normal RivetKit entrypoints (`registry.start()`, `registry.serve()`, or
 * `registry.handler(req)`). `createRivetWorld` returns a World instance that
 * talks to this registry over a RivetKit client.
 */

import { setup } from "rivetkit";
import { coordinatorActor } from "./actors/coordinator";
import { queueActor } from "./actors/queue";
import { workflowRunActor } from "./actors/workflow-run";

export const registry = setup({
	use: {
		workflowRun: workflowRunActor,
		coordinator: coordinatorActor,
		queueRunner: queueActor,
	},
});

export type WorldRegistry = typeof registry;

export { workflowRunActor, coordinatorActor, queueActor };

/**
 * Workflow registrations and the handle a workflow execution exposes to the
 * engine while it runs inside its actor.
 *
 * Portions adapted from Effect (MIT License,
 * Copyright (c) 2023 Effectful Technologies Inc). See `LICENSE-effect`.
 *
 * @internal
 */
import * as Context from "effect/Context";
import type * as Effect from "effect/Effect";
import type * as Workflow from "effect/workflow/Workflow";
import type * as WorkflowEngine from "effect/workflow/WorkflowEngine";
import type { ExecutionParent, WakeMode } from "./workflowStorage.ts";

/** @internal */
export interface WorkflowRegistration {
	readonly workflow: Workflow.Any;
	readonly execute: (
		payload: object,
		executionId: string,
	) => Effect.Effect<
		unknown,
		unknown,
		WorkflowEngine.WorkflowInstance | WorkflowEngine.WorkflowEngine
	>;
	readonly context: Context.Context<never>;
}

/** @internal */
export interface WorkflowRegistry {
	readonly get: (name: string) => WorkflowRegistration | undefined;
	readonly names: () => ReadonlyArray<string>;
	/** Returns `false` when the name already has a registration. */
	readonly register: (
		name: string,
		registration: WorkflowRegistration,
	) => boolean;
	readonly unregister: (
		name: string,
		registration: WorkflowRegistration,
	) => void;
}

/** @internal */
export const makeWorkflowRegistry = (): WorkflowRegistry => {
	const registrations = new Map<string, WorkflowRegistration>();
	return {
		get: (name) => registrations.get(name),
		names: () => Array.from(registrations.keys()),
		register: (name, registration) => {
			if (registrations.has(name)) return false;
			registrations.set(name, registration);
			return true;
		},
		unregister: (name, registration) => {
			if (registrations.get(name) === registration)
				registrations.delete(name);
		},
	};
};

/** @internal */
export interface WorkflowRunOptions {
	readonly discard: boolean;
	readonly parent?: ExecutionParent | undefined;
}

/** The actions of a workflow actor, as the engine calls them. */
export interface WorkflowStub {
	readonly run: (
		payload: string,
		options: WorkflowRunOptions,
	) => Promise<string>;
	readonly poll: () => Promise<string | undefined>;
	readonly resume: (mode?: WakeMode) => Promise<void>;
	readonly interrupt: () => Promise<void>;
	readonly interruptUnsafe: () => Promise<void>;
	readonly deferredDone: (name: string, exit: string) => Promise<void>;
	readonly scheduleClock: (
		name: string,
		deferredName: string,
		wakeUp: number,
	) => Promise<void>;
}

/** The running execution's own handle, used to avoid a call to itself. */
export interface WorkflowExecutionHandle extends WorkflowStub {
	readonly executionId: string;
	readonly loadActivity: (key: string) => Effect.Effect<string | undefined>;
	readonly saveActivity: (key: string, exit: string) => Effect.Effect<void>;
	readonly loadDeferred: (name: string) => Effect.Effect<string | undefined>;
	readonly completeOwnDeferred: (
		name: string,
		exit: string,
	) => Effect.Effect<void>;
}

/** @internal */
export class CurrentExecutionHandle extends Context.Service<
	CurrentExecutionHandle,
	WorkflowExecutionHandle
>()("@rivetkit/effect-cluster/RivetWorkflowEngine/CurrentExecutionHandle") {}

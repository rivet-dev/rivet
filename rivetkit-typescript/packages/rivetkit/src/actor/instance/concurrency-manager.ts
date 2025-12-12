import { stringifyError } from "@/common/utils";
import {
	isWrappedDefinition,
	type Concurrency,
	type ActorConfig,
	type HookOptions,
	type WrappedDefinition,
} from "../config";
import type { Conn } from "../conn/mod";
import { ActionContext } from "../contexts/action";
import type { AnyDatabaseProvider } from "../database";
import * as errors from "../errors";
import { DeadlineError, deadline } from "../utils";
import type { ActorInstance } from "./mod";

/**
 * Manages action and hook execution with concurrency control.
 * Handles serial, parallel, and readonly concurrency modes.
 */
export class ConcurrencyManager<S, CP, CS, V, I, DB extends AnyDatabaseProvider> {
	#actor: ActorInstance<S, CP, CS, V, I, DB>;
	#config: ActorConfig<S, CP, CS, V, I, DB>;

	/** Currently executing serial operation promise, if any */
	#runningSerial: Promise<unknown> | null = null;
	/** Currently executing parallel operation promises */
	#runningParallel: Set<Promise<unknown>> = new Set();

	constructor(
		actor: ActorInstance<S, CP, CS, V, I, DB>,
		config: ActorConfig<S, CP, CS, V, I, DB>,
	) {
		this.#actor = actor;
		this.#config = config;
	}

	// MARK: - Core Execution

	/**
	 * Core execution method that handles concurrency for any async operation.
	 *
	 * @param concurrency - The concurrency mode
	 * @param executor - Function that performs the actual work
	 * @param timeout - Optional timeout in milliseconds
	 * @returns The executor's return value
	 */
	async #executeWithConcurrency<T>(
		concurrency: Concurrency,
		executor: () => T | Promise<T>,
		timeout?: number,
	): Promise<T> {
		// Wait for appropriate operations based on concurrency mode
		await this.#waitForConcurrency(concurrency);

		// Create the execution promise
		const executionPromise = (async () => {
			const result = executor();
			if (result instanceof Promise) {
				if (timeout !== undefined) {
					return await deadline(result, timeout);
				}
				return await result;
			}
			return result;
		})();

		// Track the promise based on concurrency mode
		if (concurrency === "serial") {
			this.#runningSerial = executionPromise;
			try {
				return await executionPromise;
			} finally {
				this.#runningSerial = null;
			}
		} else if (concurrency === "parallel") {
			this.#runningParallel.add(executionPromise);
			try {
				return await executionPromise;
			} finally {
				this.#runningParallel.delete(executionPromise);
			}
		} else {
			// readonly: not tracked at all
			return await executionPromise;
		}
	}

	/**
	 * Waits for the appropriate operations to complete based on concurrency mode.
	 */
	async #waitForConcurrency(concurrency: Concurrency): Promise<void> {
		if (concurrency === "readonly") {
			// readonly: does not wait for anything
			return;
		}

		if (concurrency === "parallel") {
			// parallel: waits for all serial operations to complete
			if (this.#runningSerial) {
				await this.#runningSerial;
			}
			return;
		}

		// serial: waits for all serial & parallel operations to complete
		const waitPromises: Promise<unknown>[] = [];
		if (this.#runningSerial) {
			waitPromises.push(this.#runningSerial);
		}
		for (const p of this.#runningParallel) {
			waitPromises.push(p);
		}
		if (waitPromises.length > 0) {
			await Promise.all(waitPromises);
		}
	}

	// MARK: - Action Execution

	/**
	 * Executes an action with proper concurrency handling.
	 *
	 * @param ctx - The action context
	 * @param actionName - Name of the action to execute
	 * @param args - Arguments to pass to the action
	 * @returns The action's return value
	 */
	async executeAction(
		ctx: ActionContext<S, CP, CS, V, I, DB>,
		actionName: string,
		args: unknown[],
	): Promise<unknown> {
		this.#actor.assertReady();

		if (!(actionName in this.#config.actions)) {
			this.#actor.rLog.warn({ msg: "action does not exist", actionName });
			throw new errors.ActionNotFound(actionName);
		}

		const actionEntry = this.#config.actions[actionName];

		// Unwrap WrappedDefinition if needed
		const actionFunction = isWrappedDefinition(actionEntry)
			? actionEntry.handler
			: actionEntry;
		const actionOptions = isWrappedDefinition(actionEntry)
			? actionEntry.options
			: undefined;

		if (typeof actionFunction !== "function") {
			this.#actor.rLog.warn({
				msg: "action is not a function",
				actionName,
				type: typeof actionFunction,
			});
			throw new errors.ActionNotFound(actionName);
		}

		// Use action-specific timeout if provided, otherwise use default
		const actionTimeout =
			actionOptions?.timeout ?? this.#config.options.actionTimeout;

		// Get concurrency mode, defaulting to "serial"
		const concurrency: Concurrency =
			actionOptions?.concurrency ?? "serial";

		try {
			return await this.#executeWithConcurrency(
				concurrency,
				() => this.#invokeAction(ctx, actionName, actionFunction, args),
				actionTimeout,
			);
		} catch (error) {
			if (error instanceof DeadlineError) {
				throw new errors.ActionTimedOut();
			}
			this.#actor.rLog.error({
				msg: "action error",
				actionName,
				error: stringifyError(error),
			});
			throw error;
		} finally {
			this.#actor.stateManager.savePersistThrottled();
		}
	}

	/**
	 * Invokes the action function and processes the response.
	 */
	async #invokeAction(
		ctx: ActionContext<S, CP, CS, V, I, DB>,
		actionName: string,
		actionFunction: (...args: any[]) => any,
		args: unknown[],
	): Promise<unknown> {
		this.#actor.rLog.debug({
			msg: "executing action",
			actionName,
			args,
		});

		let output = actionFunction.call(undefined, ctx, ...args);
		if (output instanceof Promise) {
			output = await output;
		}

		// Process through onBeforeActionResponse if configured
		if (this.#config.onBeforeActionResponse) {
			output = await this.#invokeOnBeforeActionResponse(
				actionName,
				args,
				output,
			);
		}

		return output;
	}

	/**
	 * Invokes the onBeforeActionResponse hook.
	 */
	async #invokeOnBeforeActionResponse(
		actionName: string,
		args: unknown[],
		output: unknown,
	): Promise<unknown> {
		const hookEntry = this.#config.onBeforeActionResponse;
		if (!hookEntry) return output;

		try {
			const handler = isWrappedDefinition(hookEntry)
				? hookEntry.handler
				: hookEntry;

			const processedOutput = handler(
				this.#actor.actorContext,
				actionName,
				args,
				output,
			);

			if (processedOutput instanceof Promise) {
				return await processedOutput;
			}
			return processedOutput;
		} catch (error) {
			this.#actor.rLog.error({
				msg: "error in `onBeforeActionResponse`",
				error: stringifyError(error),
			});
			return output;
		}
	}

	/**
	 * Executes a scheduled action with proper concurrency handling.
	 * Creates a temporary internal connection for the scheduled action.
	 *
	 * @param createConn - Function to create a temporary connection
	 * @param actionName - Name of the action to execute
	 * @param args - Arguments to pass to the action
	 * @returns The action's return value
	 */
	async executeScheduledAction(
		createConn: () => Promise<Conn<S, CP, CS, V, I, DB>>,
		actionName: string,
		args: unknown[],
	): Promise<unknown> {
		const conn = await createConn();

		try {
			const ctx = new ActionContext(this.#actor, conn);
			return await this.executeAction(ctx, actionName, args);
		} finally {
			conn.disconnect();
		}
	}

	// MARK: - Hook Execution

	/**
	 * Executes a hook with proper concurrency handling.
	 *
	 * @param hookEntry - The hook entry (function or HookDefinition)
	 * @param invoker - Function that invokes the hook handler
	 * @param defaultConcurrency - Default concurrency mode if not specified
	 */
	async executeHook<THandler extends (...args: any[]) => any>(
		hookEntry: THandler | WrappedDefinition<THandler, HookOptions> | undefined,
		invoker: (handler: THandler) => unknown,
		defaultConcurrency: Concurrency = "serial",
	): Promise<void> {
		if (!hookEntry) return;

		const handler = isWrappedDefinition(hookEntry)
			? hookEntry.handler
			: hookEntry;
		const options = isWrappedDefinition(hookEntry)
			? hookEntry.options
			: undefined;
		const concurrency = (options as HookOptions | undefined)?.concurrency ?? defaultConcurrency;

		await this.#executeWithConcurrency(concurrency, async () => {
			const result = invoker(handler as THandler);
			if (result instanceof Promise) {
				await result;
			}
		});
	}

	/**
	 * Executes a hook that returns a value with proper concurrency handling.
	 *
	 * @param hookEntry - The hook entry (function or HookDefinition)
	 * @param invoker - Function that invokes the hook handler
	 * @param defaultConcurrency - Default concurrency mode if not specified
	 * @returns The hook's return value
	 */
	async executeHookWithReturn<THandler extends (...args: any[]) => any, TReturn>(
		hookEntry: THandler | WrappedDefinition<THandler, HookOptions>,
		invoker: (handler: THandler) => TReturn | Promise<TReturn>,
		defaultConcurrency: Concurrency = "serial",
	): Promise<TReturn> {
		const handler = isWrappedDefinition(hookEntry)
			? hookEntry.handler
			: hookEntry;
		const options = isWrappedDefinition(hookEntry)
			? hookEntry.options
			: undefined;
		const concurrency = (options as HookOptions | undefined)?.concurrency ?? defaultConcurrency;

		return await this.#executeWithConcurrency(concurrency, async () => {
			const result = invoker(handler as THandler);
			if (result instanceof Promise) {
				return await result;
			}
			return result;
		});
	}
}

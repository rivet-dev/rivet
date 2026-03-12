import type { WorkflowError, WorkflowErrorEvent } from "./types.js";

const WORKFLOW_ERROR_REPORTED_SYMBOL = Symbol("workflow.error.reported");

/**
 * Extract structured error information from an error.
 */
export function extractErrorInfo(error: unknown): WorkflowError {
	if (error instanceof Error) {
		const result: WorkflowError = {
			name: error.name,
			message: error.message,
			stack: error.stack,
		};

		const metadata: Record<string, unknown> = {};
		for (const key of Object.keys(error)) {
			if (key !== "name" && key !== "message" && key !== "stack") {
				const value = (error as unknown as Record<string, unknown>)[
					key
				];
				if (
					typeof value === "string" ||
					typeof value === "number" ||
					typeof value === "boolean" ||
					value === null
				) {
					metadata[key] = value;
				}
			}
		}
		if (Object.keys(metadata).length > 0) {
			result.metadata = metadata;
		}

		return result;
	}

	return {
		name: "Error",
		message: String(error),
	};
}

/**
 * Mark an error after it has been reported to the error hook.
 */
export function markErrorReported<T extends Error>(error: T): T {
	(
		error as T & {
			[WORKFLOW_ERROR_REPORTED_SYMBOL]?: boolean;
		}
	)[WORKFLOW_ERROR_REPORTED_SYMBOL] = true;
	return error;
}

/**
 * Check if an error was already reported to the error hook.
 */
export function isErrorReported(error: unknown): boolean {
	if (!(error instanceof Error)) {
		return false;
	}

	return Boolean(
		(
			error as Error & {
				[WORKFLOW_ERROR_REPORTED_SYMBOL]?: boolean;
			}
		)[WORKFLOW_ERROR_REPORTED_SYMBOL],
	);
}

/**
 * Return the outer tag name for a workflow error event.
 */
export function getErrorEventTag(
	event: WorkflowErrorEvent,
): "step" | "rollback" | "workflow" {
	if ("step" in event) {
		return "step";
	}
	if ("rollback" in event) {
		return "rollback";
	}
	return "workflow";
}

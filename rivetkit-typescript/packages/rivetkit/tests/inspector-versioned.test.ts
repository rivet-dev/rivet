import { describe, expect, test } from "vitest";
import type { WorkflowHistory } from "@/common/bare/transport/v1";
import { CURRENT_VERSION } from "@/common/inspector-versioned";
import {
	TO_CLIENT_VERSIONED,
	TO_SERVER_VERSIONED,
} from "@/common/inspector-versioned";
import {
	decodeWorkflowHistoryTransport,
	encodeWorkflowHistoryTransport,
} from "@/common/inspector-transport";

function buffer(text: string): ArrayBuffer {
	return new TextEncoder().encode(text).buffer;
}

describe("inspector versioned protocol", () => {
	test("tracks v4 as the current inspector wire version", () => {
		expect(CURRENT_VERSION).toBe(4);
	});

	test("round-trips a shared request shape across versions 1-4", () => {
		const request = {
			body: {
				tag: "ActionRequest" as const,
				val: {
					id: 7n,
					name: "increment",
					args: buffer("payload"),
				},
			},
		};

		for (const version of [1, 2, 3, 4]) {
			const bytes = TO_SERVER_VERSIONED.serializeWithEmbeddedVersion(
				request,
				version,
			);
			const decoded =
				TO_SERVER_VERSIONED.deserializeWithEmbeddedVersion(bytes);

			expect(decoded).toEqual(request);
		}
	});

	test("backfills v1 init messages into the current snapshot shape", () => {
		const snapshot = {
			body: {
				tag: "Init" as const,
				val: {
					connections: [{ id: "conn-1", details: buffer("conn") }],
					state: buffer("state"),
					isStateEnabled: true,
					rpcs: ["increment", "getCount"],
					isDatabaseEnabled: true,
					queueSize: 5n,
					workflowHistory: buffer("workflow"),
					isWorkflowEnabled: true,
				},
			},
		};

		const bytes = TO_CLIENT_VERSIONED.serializeWithEmbeddedVersion(
			snapshot,
			1,
		);
		const decoded =
			TO_CLIENT_VERSIONED.deserializeWithEmbeddedVersion(bytes);

		expect(decoded).toEqual({
			body: {
				tag: "Init",
				val: {
					connections: [{ id: "conn-1", details: buffer("conn") }],
					state: buffer("state"),
					isStateEnabled: true,
					rpcs: ["increment", "getCount"],
					isDatabaseEnabled: true,
					queueSize: 0n,
					workflowHistory: null,
					isWorkflowEnabled: false,
				},
			},
		});
	});

	test("downgrades dropped v1 event streams into explicit errors", () => {
		const v1EventBytes = TO_CLIENT_VERSIONED.serializeWithEmbeddedVersion(
			{
				body: {
					tag: "EventsUpdated" as const,
					val: {
						events: [
							{
								id: "event-1",
								timestamp: 123n,
								body: {
									tag: "BroadcastEvent" as const,
									val: {
										eventName: "counter.updated",
										args: buffer("payload"),
									},
								},
							},
						],
					},
				},
			},
			1,
		);
		const decoded =
			TO_CLIENT_VERSIONED.deserializeWithEmbeddedVersion(v1EventBytes);

		expect(decoded).toEqual({
			body: {
				tag: "Error",
				val: {
					message: "inspector.events_dropped",
				},
			},
		});
	});

	test("rejects workflow replay requests before v4", () => {
		expect(() =>
			TO_SERVER_VERSIONED.serializeWithEmbeddedVersion(
				{
					body: {
						tag: "WorkflowReplayRequest" as const,
						val: {
							id: 99n,
							entryId: "entry-1",
						},
					},
				},
				1,
			),
		).toThrow("Cannot convert v4-only workflow replay requests to v3");
	});
});

describe("inspector workflow transport", () => {
	test("round-trips workflow history bytes through the transport helper", () => {
		const history: WorkflowHistory = {
			nameRegistry: ["root", "child"],
			entries: [
				{
					id: "entry-1",
					location: [{ tag: "WorkflowNameIndex", val: 0 }],
					kind: {
						tag: "WorkflowStepEntry",
						val: {
							output: buffer("done"),
							error: null,
						},
					},
				},
			],
			entryMetadata: new Map([
				[
					"entry-1",
					{
						status: "COMPLETED",
						error: null,
						attempts: 1,
						lastAttemptAt: 10n,
						createdAt: 5n,
						completedAt: 10n,
						rollbackCompletedAt: null,
						rollbackError: null,
					},
				],
			]),
		};

		const encoded = encodeWorkflowHistoryTransport(history);
		const decoded = decodeWorkflowHistoryTransport(encoded);

		expect(decoded).toEqual(history);
	});
});

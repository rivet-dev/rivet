import { beforeAll, describe, expect, test } from "vitest";
import * as v1 from "@/common/bare/generated/inspector/v1";
import * as v2 from "@/common/bare/generated/inspector/v2";
import * as v3 from "@/common/bare/generated/inspector/v3";
import * as v4 from "@/common/bare/generated/inspector/v4";
import * as v5 from "@/common/bare/generated/inspector/v5";
import * as v6 from "@/common/bare/generated/inspector/v6";
import type { WorkflowHistory } from "@/common/bare/transport/v1";
import {
	decodeWorkflowHistoryTransport,
	encodeWorkflowHistoryTransport,
} from "@/common/inspector-transport";
import { loadNapiRuntime, type NapiCoreRuntime } from "@/registry/napi-runtime";

const INSPECTOR_CURRENT_VERSION = 6;
let runtime: NapiCoreRuntime;

beforeAll(async () => {
	({ runtime } = await loadNapiRuntime());
});

function buffer(text: string): ArrayBuffer {
	return new TextEncoder().encode(text).buffer;
}

function toBuffer(value: ArrayBuffer | Uint8Array): Buffer {
	return Buffer.from(
		value instanceof Uint8Array ? value : new Uint8Array(value),
	);
}

function decodeRequest(bytes: Uint8Array, version: number): v6.ToServer {
	return v6.decodeToServer(
		new Uint8Array(
			runtime.decodeInspectorRequest(toBuffer(bytes), version),
		),
	);
}

function encodeResponse(
	message: v6.ToClient,
	version: 1 | 2 | 3 | 4 | 5,
): Uint8Array {
	return new Uint8Array(
		runtime.encodeInspectorResponse(
			toBuffer(v6.encodeToClient(message)),
			version,
		),
	);
}

describe("inspector versioned protocol", () => {
	test("tracks v6 as the current inspector wire version", () => {
		expect(INSPECTOR_CURRENT_VERSION).toBe(6);
	});

	test("decodes a shared request shape from versions 1-5 into the current schema", () => {
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

		for (const version of [1, 2, 3, 4, 5]) {
			const bytes =
				version === 1
					? v1.encodeToServer(request as unknown as v1.ToServer)
					: version === 2
						? v2.encodeToServer(request as unknown as v2.ToServer)
						: version === 3
							? v3.encodeToServer(
									request as unknown as v3.ToServer,
								)
							: version === 4
								? v4.encodeToServer(request)
								: v5.encodeToServer(request);
			const decoded = decodeRequest(bytes, version);

			expect(decoded).toEqual(request);
		}
	});

	test("downgrades init snapshots into the v1 wire shape", () => {
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
					tabConfig: [],
					schedules: [],
				},
			},
		};

		const decoded = v1.decodeToClient(encodeResponse(snapshot, 1));

		expect(decoded).toEqual({
			body: {
				tag: "Init",
				val: {
					connections: [{ id: "conn-1", details: buffer("conn") }],
					events: [],
					state: buffer("state"),
					isStateEnabled: true,
					rpcs: ["increment", "getCount"],
					isDatabaseEnabled: true,
				},
			},
		});
	});

	test("downgrades schedule updates into explicit errors before v6", () => {
		const decoded = v5.decodeToClient(
			encodeResponse(
				{
					body: {
						tag: "SchedulesUpdated",
						val: { schedules: [] },
					},
				},
				5,
			),
		);
		expect(decoded).toEqual({
			body: {
				tag: "Error",
				val: { message: "inspector.schedules_dropped" },
			},
		});
	});

	test("downgrades dropped v1 queue updates into explicit errors", () => {
		const decoded = v1.decodeToClient(
			encodeResponse(
				{
					body: {
						tag: "QueueUpdated" as const,
						val: {
							queueSize: 5n,
						},
					},
				},
				1,
			),
		);

		expect(decoded).toEqual({
			body: {
				tag: "Error",
				val: {
					message: "inspector.queue_dropped",
				},
			},
		});
	});

	test("downgrades workflow replay responses into explicit errors before v4", () => {
		const decoded = v3.decodeToClient(
			encodeResponse(
				{
					body: {
						tag: "WorkflowReplayResponse" as const,
						val: {
							rid: 11n,
							history: buffer("workflow"),
							isWorkflowEnabled: true,
						},
					},
				},
				3,
			),
		);

		expect(decoded).toEqual({
			body: {
				tag: "Error",
				val: {
					message: "inspector.workflow_history_dropped",
				},
			},
		});
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
				{
					id: "entry-2",
					location: [{ tag: "WorkflowNameIndex", val: 1 }],
					kind: {
						tag: "WorkflowVersionCheckEntry",
						val: {
							resolved: 1,
							latest: 2,
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

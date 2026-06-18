/**
 * Regression test for rivetkit issue #6:
 *   "No hook to inject custom VFS mounts on VM creation (ensureVm)"
 *
 * The supported public surface for `agentOs(...)` lets callers pass native
 * sidecar mounts via `options.mounts`. Plain/Overlay mounts carry live JS
 * objects and are rejected until the NAPI callback channel exists.
 *
 * However, VM creation is now fully owned by the Rust crate. The only path
 * from TS into VM creation is `buildConfigJson(parsed)` (src/agent-os/actor/
 * index.ts), whose JSON envelope is consumed by the Rust deserializer
 * (AgentOsConfigJson, deny_unknown_fields). That envelope must preserve the
 * native mount descriptors needed at VM creation.
 *
 * EXPECTED (correct/fixed) BEHAVIOR encoded below: a native `mounts` entry
 * supplied via the public `options` should be forwarded into the config
 * envelope, while non-serializable mount variants fail loudly.
 */

import { describe, expect, test } from "vitest";
import { buildConfigJson } from "@/agent-os/actor/index";
import type { AgentOsActorConfig } from "@/agent-os/config";

describe("custom VFS mount injection at VM creation", () => {
	test("buildConfigJson forwards the serializable agent-os config surface", () => {
		const parsed = {
			options: {
				software: [],
				additionalInstructions: "Be concise.",
				loopbackExemptPorts: [3000, 5173],
				allowedNodeBuiltins: ["fs", "path"],
				permissions: {
					fs: "deny",
					network: "allow",
				},
				rootFilesystem: {
					mode: "read-only",
					disableDefaultBaseLayer: true,
				},
				limits: {
					resources: {
						maxProcesses: 4,
					},
				},
				sidecar: {
					kind: "shared",
					pool: "zid",
				},
			},
			preview: {
				defaultExpiresInSeconds: 3600,
				maxExpiresInSeconds: 86400,
			},
		} as unknown as AgentOsActorConfig<undefined>;

		const envelope = JSON.parse(buildConfigJson(parsed));

		expect(envelope).toMatchObject({
			additionalInstructions: "Be concise.",
			loopbackExemptPorts: [3000, 5173],
			allowedNodeBuiltins: ["fs", "path"],
			permissions: {
				fs: "deny",
				network: "allow",
			},
			rootFilesystem: {
				mode: "read-only",
				disableDefaultBaseLayer: true,
			},
			limits: {
				resources: {
					maxProcesses: 4,
				},
			},
			sidecar: {
				pool: "zid",
			},
		});
	});

	test("buildConfigJson forwards native options.mounts into the config envelope", () => {
		const parsed = {
			options: {
				software: [],
				mounts: [
					{
						path: "/home/user/.pi/agent/sessions",
						plugin: {
							id: "host_dir",
							config: {
								hostPath: "/tmp/agent-sessions",
								readOnly: false,
							},
						},
						readOnly: false,
					},
				],
			},
			preview: {
				defaultExpiresInSeconds: 3600,
				maxExpiresInSeconds: 86400,
			},
		} as unknown as AgentOsActorConfig<undefined>;

		const envelope = JSON.parse(buildConfigJson(parsed));

		// The public `mounts` option must survive into the envelope that
		// drives VM creation.
		expect(envelope).toHaveProperty("mounts");
		expect(Array.isArray(envelope.mounts)).toBe(true);
		expect(envelope.mounts).toEqual(
			expect.arrayContaining([
				expect.objectContaining({
					path: "/home/user/.pi/agent/sessions",
					plugin: expect.objectContaining({
						id: "host_dir",
						config: expect.objectContaining({
							hostPath: "/tmp/agent-sessions",
						}),
					}),
				}),
			]),
		);
	});

	test("buildConfigJson rejects plain driver mounts because callbacks cannot cross JSON", () => {
		const parsed = {
			options: {
				mounts: [
					{
						path: "/home/user/.pi/agent/sessions",
						driver: {
							read: () => undefined,
						},
					},
				],
			},
			preview: {
				defaultExpiresInSeconds: 3600,
				maxExpiresInSeconds: 86400,
			},
		} as unknown as AgentOsActorConfig<undefined>;

		expect(() => buildConfigJson(parsed)).toThrow(
			/Plain mounts|not serializable/i,
		);
	});
});

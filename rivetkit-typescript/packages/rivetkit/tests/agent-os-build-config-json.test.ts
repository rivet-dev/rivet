// Verifies the TS shim's `buildConfigJson` produces the JSON envelope
// the NAPI `AgentOsConfigJson` deserializer expects, with `kind`
// inferred from the JS descriptor shape (the user never specifies it).
//
// The Rust-side counterpart lives at
// `rivetkit-napi/tests/agent_os_factory.rs::parsing` and verifies the
// same envelope round-trips into a working `AgentOsConfig`.

import { describe, expect, test } from "vitest";
import { buildConfigJson } from "../src/agent-os/actor/index";
import type { AgentOsActorConfig } from "../src/agent-os/config";

function makeConfig(options: unknown): AgentOsActorConfig {
	return {
		options: options as never,
		preview: { defaultExpiresInSeconds: 3600, maxExpiresInSeconds: 86400 },
	} as AgentOsActorConfig;
}

describe("agentOs buildConfigJson — kind inference", () => {
	test("empty config produces empty object", () => {
		expect(JSON.parse(buildConfigJson(makeConfig(undefined)))).toEqual({});
	});

	test("no software field produces empty object", () => {
		expect(JSON.parse(buildConfigJson(makeConfig({})))).toEqual({});
	});

	test("descriptor with commandDir is inferred as wasm-commands", () => {
		const json = buildConfigJson(
			makeConfig({
				software: [
					{
						name: "coreutils",
						type: "wasm-commands",
						commandDir: "/abs/path/coreutils/wasm",
					},
				],
			}),
		);
		expect(JSON.parse(json)).toEqual({
			software: [
				{ package: "/abs/path/coreutils/wasm", kind: "wasm-commands" },
			],
		});
	});

	test("duck-typed wasm package (no type field, only commandDir) is wasm-commands", () => {
		// Matches the `WasmCommandDirDescriptor` shape used by
		// `@rivet-dev/agent-os-common`'s entries.
		const json = buildConfigJson(
			makeConfig({
				software: [
					{
						name: "coreutils",
						aptName: "coreutils",
						source: "rust",
						commandDir: "/abs/path/coreutils/wasm",
					},
				],
			}),
		);
		expect(JSON.parse(json)).toEqual({
			software: [
				{ package: "/abs/path/coreutils/wasm", kind: "wasm-commands" },
			],
		});
	});

	test("agent descriptor maps to kind=agent with packageDir", () => {
		const json = buildConfigJson(
			makeConfig({
				software: [
					{
						name: "pi",
						type: "agent",
						packageDir: "/abs/path/agent-os-pi",
						requires: [],
						agent: { id: "pi", acpAdapter: "pi-acp", agentPackage: "pi" },
					},
				],
			}),
		);
		expect(JSON.parse(json)).toEqual({
			software: [{ package: "/abs/path/agent-os-pi", kind: "agent" }],
		});
	});

	test("tool descriptor maps to kind=tool with packageDir", () => {
		const json = buildConfigJson(
			makeConfig({
				software: [
					{
						name: "some-tool",
						type: "tool",
						packageDir: "/abs/path/some-tool",
						requires: [],
						bins: { foo: "some-bin" },
					},
				],
			}),
		);
		expect(JSON.parse(json)).toEqual({
			software: [{ package: "/abs/path/some-tool", kind: "tool" }],
		});
	});

	test("meta-package (nested array) is shallow-flattened with per-entry inference", () => {
		const json = buildConfigJson(
			makeConfig({
				software: [
					[
						{ name: "coreutils", commandDir: "/abs/coreutils/wasm" },
						{ name: "grep", commandDir: "/abs/grep/wasm" },
						{ name: "sed", commandDir: "/abs/sed/wasm" },
					],
				],
			}),
		);
		expect(JSON.parse(json)).toEqual({
			software: [
				{ package: "/abs/coreutils/wasm", kind: "wasm-commands" },
				{ package: "/abs/grep/wasm", kind: "wasm-commands" },
				{ package: "/abs/sed/wasm", kind: "wasm-commands" },
			],
		});
	});

	test("mixed array preserves order and infers each entry independently", () => {
		const json = buildConfigJson(
			makeConfig({
				software: [
					{
						name: "pi",
						type: "agent",
						packageDir: "/abs/pi",
						requires: [],
						agent: { id: "pi", acpAdapter: "a", agentPackage: "p" },
					},
					[
						{ name: "coreutils", commandDir: "/abs/coreutils/wasm" },
						{ name: "grep", commandDir: "/abs/grep/wasm" },
					],
					{
						name: "some-tool",
						type: "tool",
						packageDir: "/abs/tool",
						requires: [],
						bins: {},
					},
				],
			}),
		);
		expect(JSON.parse(json)).toEqual({
			software: [
				{ package: "/abs/pi", kind: "agent" },
				{ package: "/abs/coreutils/wasm", kind: "wasm-commands" },
				{ package: "/abs/grep/wasm", kind: "wasm-commands" },
				{ package: "/abs/tool", kind: "tool" },
			],
		});
	});

	test("malformed entries (no commandDir, no packageDir) are silently dropped", () => {
		const json = buildConfigJson(
			makeConfig({
				software: [
					{ name: "valid", commandDir: "/abs/valid" },
					{ name: "noPaths" },
					{ name: "wrongType", type: "agent" }, // missing packageDir
					null,
					undefined,
					42,
					"string-not-object",
					{ name: "another-valid", commandDir: "/abs/another" },
				],
			}),
		);
		expect(JSON.parse(json)).toEqual({
			software: [
				{ package: "/abs/valid", kind: "wasm-commands" },
				{ package: "/abs/another", kind: "wasm-commands" },
			],
		});
	});

	test("commandDir takes precedence even if type is set to agent/tool", () => {
		// Edge case: if a descriptor somehow carries both, the wasm signal
		// wins because that's what the discriminator says when commandDir
		// is present (matches the legacy TS port's behavior).
		const json = buildConfigJson(
			makeConfig({
				software: [
					{
						name: "weird",
						type: "agent",
						commandDir: "/abs/weird/wasm",
						packageDir: "/abs/weird-pkg",
					},
				],
			}),
		);
		expect(JSON.parse(json)).toEqual({
			software: [{ package: "/abs/weird/wasm", kind: "wasm-commands" }],
		});
	});

	test("additionalInstructions / moduleAccessCwd round-trip when set", () => {
		const json = buildConfigJson(
			makeConfig({
				additionalInstructions: "Be terse.",
				moduleAccessCwd: "/home/user/workspace",
			}),
		);
		expect(JSON.parse(json)).toEqual({
			additionalInstructions: "Be terse.",
			moduleAccessCwd: "/home/user/workspace",
		});
	});

	test("loopbackExemptPorts + allowedNodeBuiltins round-trip as filtered arrays", () => {
		const json = buildConfigJson(
			makeConfig({
				loopbackExemptPorts: [9000, "not-a-port", 9001],
				allowedNodeBuiltins: ["fs", 42, "path"],
			}),
		);
		expect(JSON.parse(json)).toEqual({
			loopbackExemptPorts: [9000, 9001],
			allowedNodeBuiltins: ["fs", "path"],
		});
	});

	test("everything-set config emits a fully populated envelope", () => {
		const json = buildConfigJson(
			makeConfig({
				software: [
					[
						{ name: "coreutils", commandDir: "/abs/coreutils/wasm" },
						{ name: "grep", commandDir: "/abs/grep/wasm" },
					],
					{
						name: "pi",
						type: "agent",
						packageDir: "/abs/pi",
						requires: [],
						agent: { id: "pi", acpAdapter: "a", agentPackage: "p" },
					},
				],
				additionalInstructions: "follow the rules",
				moduleAccessCwd: "/workspace",
				loopbackExemptPorts: [8080],
				allowedNodeBuiltins: ["fs"],
			}),
		);
		expect(JSON.parse(json)).toEqual({
			software: [
				{ package: "/abs/coreutils/wasm", kind: "wasm-commands" },
				{ package: "/abs/grep/wasm", kind: "wasm-commands" },
				{ package: "/abs/pi", kind: "agent" },
			],
			additionalInstructions: "follow the rules",
			moduleAccessCwd: "/workspace",
			loopbackExemptPorts: [8080],
			allowedNodeBuiltins: ["fs"],
		});
	});
});

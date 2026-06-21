/// Phase 1b E2E gate. Verifies that `NapiActorFactory.fromAgentOs` returns
/// a non-null handle for valid config and fails loud for unknown fields.
///
/// This does NOT bring up an agent-os VM — that's Phase 1c's full-driver
/// gate. Phase 1b only proves the JS → NAPI factory construction path.

import { describe, expect, test } from "vitest";
import { NapiActorFactory } from "@rivetkit/rivetkit-napi";

describe("NapiActorFactory.fromAgentOs (Phase 1b)", () => {
	test("returns a handle when given a valid empty config", () => {
		const factory = NapiActorFactory.fromAgentOs(
			{ configJson: "{}" },
			undefined,
		);
		expect(factory).toBeDefined();
	});

	test("returns a handle when configJson is omitted", () => {
		const factory = NapiActorFactory.fromAgentOs({}, undefined);
		expect(factory).toBeDefined();
	});

	test("returns a handle for a software-only config", () => {
		const configJson = JSON.stringify({
			software: [{ package: "node" }],
		});
		const factory = NapiActorFactory.fromAgentOs({ configJson }, undefined);
		expect(factory).toBeDefined();
	});

	test("fails loud on unknown top-level field (driver)", () => {
		// `driver` is a non-serializable AgentOsConfig field that must
		// never come in via JSON. `deny_unknown_fields` rejects it.
		const configJson = JSON.stringify({
			software: [{ package: "node" }],
			driver: "some-driver",
		});
		expect(() =>
			NapiActorFactory.fromAgentOs({ configJson }, undefined),
		).toThrow(/configJson|driver|unknown field/i);
	});

	test("fails loud on malformed JSON", () => {
		expect(() =>
			NapiActorFactory.fromAgentOs(
				{ configJson: "{not valid json" },
				undefined,
			),
		).toThrow(/configJson|parse|expected/i);
	});

	test("fails loud on non-serializable schedule_driver field", () => {
		// schedule_driver is `Arc<dyn ScheduleDriver>` — explicitly absent
		// from the serializable subset.
		const configJson = JSON.stringify({
			scheduleDriver: { kind: "timer" },
		});
		expect(() =>
			NapiActorFactory.fromAgentOs({ configJson }, undefined),
		).toThrow(/configJson|scheduleDriver|unknown field/i);
	});
});

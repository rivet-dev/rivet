import { describe, expect, test } from "vitest";
import { joinSignals } from "./utils";

describe("joinSignals", () => {
	test("returns a non-aborted signal when no inputs are provided", () => {
		const signal = joinSignals();
		expect(signal.aborted).toBe(false);
	});

	test("returns the same signal when only one input is provided", () => {
		const controller = new AbortController();
		const signal = joinSignals(controller.signal);
		expect(signal).toBe(controller.signal);
	});

	test("aborts immediately when any input signal is already aborted", () => {
		const controllerA = new AbortController();
		const controllerB = new AbortController();
		controllerA.abort("already-aborted");

		const signal = joinSignals(controllerA.signal, controllerB.signal);
		expect(signal.aborted).toBe(true);
	});

	test("aborts when one of the joined signals aborts", () => {
		const controllerA = new AbortController();
		const controllerB = new AbortController();
		const signal = joinSignals(controllerA.signal, controllerB.signal);

		expect(signal.aborted).toBe(false);
		controllerB.abort("stopped");
		expect(signal.aborted).toBe(true);
	});
});

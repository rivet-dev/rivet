import { describe } from "vitest";
import type { DriverTestConfig } from "../mod";
import { runActorLifecycleTests } from "./actor-lifecycle";
import { runActorScheduleTests } from "./actor-schedule";
import { runActorSleepTests } from "./actor-sleep";
import { runActorStateTests } from "./actor-state";

export function runActorDriverTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Driver Tests", () => {
		// Run state persistence tests
		runActorStateTests(driverTestConfig);

		// Run scheduled alarms tests
		runActorScheduleTests(driverTestConfig);

		// Run actor sleep tests
		runActorSleepTests(driverTestConfig);

		// Run actor lifecycle tests
		runActorLifecycleTests(driverTestConfig);
	});
}

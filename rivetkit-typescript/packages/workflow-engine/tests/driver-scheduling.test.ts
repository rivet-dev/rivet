import { beforeEach, describe, expect, it } from "vitest";
import { InMemoryDriver } from "../src/testing.js";

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(
		`Workflow Engine Driver Scheduling (${mode})`,
		{ sequential: true },
		() => {
			let driver: InMemoryDriver;

			beforeEach(() => {
				driver = new InMemoryDriver();
				driver.latency = 0;
			});

			it("should set and clear alarms", async () => {
				const wakeAt = Date.now() + 1000;

				await driver.setAlarm("wf-1", wakeAt);
				expect(driver.getAlarm("wf-1")).toBe(wakeAt);

				await driver.clearAlarm("wf-1");
				expect(driver.getAlarm("wf-1")).toBeUndefined();
			});

			it("should return due alarms", async () => {
				await driver.setAlarm("wf-due", Date.now() - 1);
				await driver.setAlarm("wf-later", Date.now() + 1000);

				const due = driver.getDueAlarms();
				expect(due).toContain("wf-due");
				expect(due).not.toContain("wf-later");
			});

			it("should expose worker poll interval", async () => {
				expect(driver.workerPollInterval).toBeGreaterThan(0);
			});
		},
	);
}

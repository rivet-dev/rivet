import { describe, expect, test, vi } from "vitest";
import { ActorContextHandleAdapter } from "@/registry/native";
import { InstanceActorOptionsSchema } from "@/actor/config";
import type {
	ActorContextHandle,
	CoreRuntime,
	RuntimeCronJobInfo,
	RuntimeScheduledEventInfo,
} from "@/registry/runtime";
import { decodeCborCompat } from "@/serde";

const context = {} as ActorContextHandle;

function runtimeWithScheduleSpies() {
	const actorCronSet = vi.fn(async (..._args: unknown[]) => {});
	const actorCronEvery = vi.fn(async (..._args: unknown[]) => {});
	const actorScheduleAfter = vi.fn(
		async (..._args: unknown[]) => "one-shot-id",
	);
	const actorScheduleGet = vi.fn(
		async (..._args: unknown[]): Promise<RuntimeScheduledEventInfo> => ({
			id: "one-shot-id",
			action: "remind",
			args: new Uint8Array([0x81, 0x64, 0x75, 0x73, 0x65, 0x72]),
			runAt: 123,
		}),
	);
	const actorCronGet = vi.fn(
		async (..._args: unknown[]): Promise<RuntimeCronJobInfo> => ({
			name: "daily-report",
			kind: "cron",
			action: "generateReport",
			args: new Uint8Array([
				0x81,
				0x6a,
				...new TextEncoder().encode("operations"),
			]),
			nextRunAt: 456,
			expression: "0 9 * * *",
			timezone: "America/Los_Angeles",
			maxHistory: 100,
		}),
	);
	const runtime = {
		kind: "napi",
		actorId: () => "cron-api-test",
		actorCronSet,
		actorCronEvery,
		actorCronGet,
		actorScheduleAfter,
		actorScheduleGet,
	} as unknown as CoreRuntime;
	return {
		runtime,
		actorCronSet,
		actorCronEvery,
		actorCronGet,
		actorScheduleAfter,
		actorScheduleGet,
	};
}

describe("actor scheduling public API", () => {
	test("schedule cap defaults to 1000 and validates nonnegative integers", () => {
		expect(InstanceActorOptionsSchema.parse({}).maxSchedules).toBe(1_000);
		expect(
			InstanceActorOptionsSchema.parse({ maxSchedules: 25 }).maxSchedules,
		).toBe(25);
		expect(() =>
			InstanceActorOptionsSchema.parse({ maxSchedules: -1 }),
		).toThrow();
		expect(() =>
			InstanceActorOptionsSchema.parse({ maxSchedules: 1.5 }),
		).toThrow();
	});

	test("set and every accept option objects and encode args arrays", async () => {
		const { runtime, actorCronSet, actorCronEvery } =
			runtimeWithScheduleSpies();
		const c = new ActorContextHandleAdapter(runtime, context);

		await c.cron.set({
			name: "daily-report",
			expression: "0 9 * * *",
			timezone: "America/Los_Angeles",
			action: "generateReport",
			args: ["operations"],
			maxHistory: 20,
		});
		await c.cron.every({
			name: "refresh-cache",
			intervalMs: 60_000,
			action: "refreshCache",
		});

		const setCall = actorCronSet.mock.calls[0];
		expect(setCall.slice(1, 5)).toEqual([
			"daily-report",
			"0 9 * * *",
			"America/Los_Angeles",
			"generateReport",
		]);
		expect(decodeCborCompat(setCall[5] as Uint8Array)).toEqual([
			"operations",
		]);
		expect(setCall[6]).toBe(20);

		const everyCall = actorCronEvery.mock.calls[0];
		expect(everyCall.slice(1, 4)).toEqual([
			"refresh-cache",
			60_000,
			"refreshCache",
		]);
		expect(decodeCborCompat(everyCall[4] as Uint8Array)).toEqual([]);
		expect(everyCall[5]).toBeUndefined();
	});

	test("one-shot and inspection values round-trip through the adapter", async () => {
		const { runtime, actorScheduleAfter, actorScheduleGet, actorCronGet } =
			runtimeWithScheduleSpies();
		const c = new ActorContextHandleAdapter(runtime, context);

		await expect(c.schedule.after(5_000, "remind", "user")).resolves.toBe(
			"one-shot-id",
		);
		expect(decodeCborCompat(actorScheduleAfter.mock.calls[0][3])).toEqual([
			"user",
		]);
		await expect(c.schedule.get("one-shot-id")).resolves.toMatchObject({
			id: "one-shot-id",
			action: "remind",
			args: ["user"],
		});
		await expect(c.cron.get("daily-report")).resolves.toMatchObject({
			name: "daily-report",
			kind: "cron",
			args: ["operations"],
		});
		expect(actorScheduleGet).toHaveBeenCalledWith(context, "one-shot-id");
		expect(actorCronGet).toHaveBeenCalledWith(context, "daily-report");
	});
});

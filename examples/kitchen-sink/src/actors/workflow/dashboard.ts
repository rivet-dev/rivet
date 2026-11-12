// DASHBOARD (Join Demo)
// Demonstrates: Parallel data fetching with join (wait-all)

import { actor, event, queue } from "rivetkit";
import type { RawAccess } from "rivetkit/db";
import { Loop, workflow } from "rivetkit/workflow";
import {
	loadRecord,
	recordDb,
	saveRecord,
	updateRecord,
} from "./record-store.ts";

export type UserStats = {
	count: number;
	activeToday: number;
	newThisWeek: number;
};

export type OrderStats = {
	count: number;
	revenue: number;
	avgOrderValue: number;
};

export type MetricsStats = {
	pageViews: number;
	sessions: number;
	bounceRate: number;
};

export type DashboardData = {
	users: UserStats;
	orders: OrderStats;
	metrics: MetricsStats;
	fetchedAt: number;
};

export type BranchStatus = "pending" | "running" | "completed" | "failed";

export type DashboardState = {
	data: DashboardData | null;
	loading: boolean;
	branches: {
		users: BranchStatus;
		orders: BranchStatus;
		metrics: BranchStatus;
	};
	lastRefresh: number | null;
};

const QUEUE_REFRESH = "refresh";
type RefreshMessage = Record<string, never>;

async function fetchUserStats(): Promise<UserStats> {
	await new Promise((r) => setTimeout(r, 800 + Math.random() * 1200));
	return {
		count: Math.floor(1000 + Math.random() * 500),
		activeToday: Math.floor(100 + Math.random() * 200),
		newThisWeek: Math.floor(20 + Math.random() * 80),
	};
}

async function fetchOrderStats(): Promise<OrderStats> {
	await new Promise((r) => setTimeout(r, 600 + Math.random() * 1000));
	const count = Math.floor(50 + Math.random() * 150);
	const revenue = Math.floor(5000 + Math.random() * 15000);
	return {
		count,
		revenue,
		avgOrderValue: Math.round(revenue / count),
	};
}

async function fetchMetricsStats(): Promise<MetricsStats> {
	await new Promise((r) => setTimeout(r, 400 + Math.random() * 800));
	return {
		pageViews: Math.floor(10000 + Math.random() * 50000),
		sessions: Math.floor(2000 + Math.random() * 8000),
		bounceRate: Math.round(30 + Math.random() * 40),
	};
}

export const dashboard = actor({
	db: recordDb(),
	onCreate: async (c) => {
		await saveRecord<DashboardState>(c.db, {
			data: null,
			loading: false,
			branches: {
				users: "pending",
				orders: "pending",
				metrics: "pending",
			},
			lastRefresh: null,
		});
	},
	queues: {
		[QUEUE_REFRESH]: queue<RefreshMessage>(),
	},
	events: {
		stateChanged: event<DashboardState>(),
		refreshComplete: event<DashboardData>(),
	},

	actions: {
		refresh: async (c) => {
			const dashboard = await startRefresh(c.db);
			if (dashboard) {
				c.broadcast("stateChanged", dashboard);
				await c.queue.send(QUEUE_REFRESH, {});
			}
		},

		getState: (c): Promise<DashboardState> =>
			loadRecord<DashboardState>(c.db),
	},

	run: workflow(async (ctx) => {
		await ctx.loop("refresh-loop", async (loopCtx) => {
			await loopCtx.queue.next("wait-refresh", {
				names: [QUEUE_REFRESH],
			});

			ctx.log.info({ msg: "starting dashboard refresh" });

			const results = await loopCtx.join("fetch-all", {
				users: {
					run: async (branchCtx) => {
						await branchCtx.step("mark-running", async (bc) => {
							const updated = await updateRecord<DashboardState>(
								bc.db,
								(dashboard) => {
									dashboard.branches.users = "running";
								},
							);
							bc.broadcast("stateChanged", updated);
						});

						const data = await branchCtx.step(
							"fetch-users",
							async () => {
								return await fetchUserStats();
							},
						);

						await branchCtx.step("mark-complete", async (bc) => {
							const updated = await updateRecord<DashboardState>(
								bc.db,
								(dashboard) => {
									dashboard.branches.users = "completed";
								},
							);
							bc.broadcast("stateChanged", updated);
						});

						return data;
					},
				},
				orders: {
					run: async (branchCtx) => {
						await branchCtx.step("mark-running", async (bc) => {
							const updated = await updateRecord<DashboardState>(
								bc.db,
								(dashboard) => {
									dashboard.branches.orders = "running";
								},
							);
							bc.broadcast("stateChanged", updated);
						});

						const data = await branchCtx.step(
							"fetch-orders",
							async () => {
								return await fetchOrderStats();
							},
						);

						await branchCtx.step("mark-complete", async (bc) => {
							const updated = await updateRecord<DashboardState>(
								bc.db,
								(dashboard) => {
									dashboard.branches.orders = "completed";
								},
							);
							bc.broadcast("stateChanged", updated);
						});

						return data;
					},
				},
				metrics: {
					run: async (branchCtx) => {
						await branchCtx.step("mark-running", async (bc) => {
							const updated = await updateRecord<DashboardState>(
								bc.db,
								(dashboard) => {
									dashboard.branches.metrics = "running";
								},
							);
							bc.broadcast("stateChanged", updated);
						});

						const data = await branchCtx.step(
							"fetch-metrics",
							async () => {
								return await fetchMetricsStats();
							},
						);

						await branchCtx.step("mark-complete", async (bc) => {
							const updated = await updateRecord<DashboardState>(
								bc.db,
								(dashboard) => {
									dashboard.branches.metrics = "completed";
								},
							);
							bc.broadcast("stateChanged", updated);
						});

						return data;
					},
				},
			});

			await loopCtx.step("save-data", async (c) => {
				const data: DashboardData = {
					users: results.users,
					orders: results.orders,
					metrics: results.metrics,
					fetchedAt: Date.now(),
				};
				const updated = await updateRecord<DashboardState>(
					c.db,
					(dashboard) => {
						dashboard.data = data;
						dashboard.loading = false;
						dashboard.lastRefresh = Date.now();
					},
				);
				c.broadcast("stateChanged", updated);
				c.broadcast("refreshComplete", data);
			});

			ctx.log.info({ msg: "dashboard refresh complete" });

			return Loop.continue(undefined);
		});
	}),
});

/**
 * Marks the dashboard as loading and resets every branch. Returns `undefined`
 * when a refresh is already in progress.
 */
async function startRefresh(
	database: RawAccess,
): Promise<DashboardState | undefined> {
	return await database.transaction(
		async (tx) => {
			const dashboard = await loadRecord<DashboardState>(tx);
			if (dashboard.loading) return undefined;
			dashboard.loading = true;
			dashboard.branches = {
				users: "pending",
				orders: "pending",
				metrics: "pending",
			};
			await saveRecord(tx, dashboard);
			return dashboard;
		},
		{ name: "start-refresh" },
	);
}

// DASHBOARD (Join Demo)
// Demonstrates: Parallel data fetching with join (wait-all)

import { actor, event, queue } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import { actorCtx } from "./_helpers.ts";

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

type State = DashboardState;

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
	state: {
		data: null as DashboardData | null,
		loading: false,
		branches: {
			users: "pending" as BranchStatus,
			orders: "pending" as BranchStatus,
			metrics: "pending" as BranchStatus,
		},
		lastRefresh: null as number | null,
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
			if (!c.state.loading) {
				c.state.loading = true;
				c.state.branches = {
					users: "pending",
					orders: "pending",
					metrics: "pending",
				};
				c.broadcast("stateChanged", c.state);
				await c.queue.send(QUEUE_REFRESH, {});
			}
		},

		getState: (c): DashboardState => c.state,
	},

	run: workflow(async (ctx) => {
		await ctx.loop("refresh-loop", async (loopCtx) => {
				const c = actorCtx<State>(loopCtx);

				await loopCtx.queue.next("wait-refresh", {
					names: [QUEUE_REFRESH],
				});

				ctx.log.info({ msg: "starting dashboard refresh" });

				const results = await loopCtx.join("fetch-all", {
					users: {
						run: async (branchCtx) => {
							const bc = actorCtx<State>(branchCtx);

							await branchCtx.step("mark-running", async () => {
								bc.state.branches.users = "running";
								bc.broadcast("stateChanged", bc.state);
							});

							const data = await branchCtx.step("fetch-users", async () => {
								return await fetchUserStats();
							});

							await branchCtx.step("mark-complete", async () => {
								bc.state.branches.users = "completed";
								bc.broadcast("stateChanged", bc.state);
							});

							return data;
						},
					},
					orders: {
						run: async (branchCtx) => {
							const bc = actorCtx<State>(branchCtx);

							await branchCtx.step("mark-running", async () => {
								bc.state.branches.orders = "running";
								bc.broadcast("stateChanged", bc.state);
							});

							const data = await branchCtx.step("fetch-orders", async () => {
								return await fetchOrderStats();
							});

							await branchCtx.step("mark-complete", async () => {
								bc.state.branches.orders = "completed";
								bc.broadcast("stateChanged", bc.state);
							});

							return data;
						},
					},
					metrics: {
						run: async (branchCtx) => {
							const bc = actorCtx<State>(branchCtx);

							await branchCtx.step("mark-running", async () => {
								bc.state.branches.metrics = "running";
								bc.broadcast("stateChanged", bc.state);
							});

							const data = await branchCtx.step("fetch-metrics", async () => {
								return await fetchMetricsStats();
							});

							await branchCtx.step("mark-complete", async () => {
								bc.state.branches.metrics = "completed";
								bc.broadcast("stateChanged", bc.state);
							});

							return data;
						},
					},
				});

				await loopCtx.step("save-data", async () => {
					c.state.data = {
						users: results.users,
						orders: results.orders,
						metrics: results.metrics,
						fetchedAt: Date.now(),
					};
					c.state.loading = false;
					c.state.lastRefresh = Date.now();
					c.broadcast("stateChanged", c.state);
					c.broadcast("refreshComplete", c.state.data);
				});

				ctx.log.info({ msg: "dashboard refresh complete" });

				return Loop.continue(undefined);
			});
	}),
});
